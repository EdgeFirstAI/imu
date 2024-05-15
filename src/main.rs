use bno08x::{
    interface::{
        delay::delay_ms,
        gpio::{GpiodIn, GpiodOut},
        spidev::SpiDevice,
        SpiInterface,
    },
    wrapper::{BNO08x, SENSOR_REPORTID_ROTATION_VECTOR},
};
use cdr::{CdrLe, Infinite};
use clap::Parser;
use edgefirst_schemas::{builtin_interfaces, geometry_msgs, sensor_msgs, std_msgs};
use log::{debug, info, trace};
use std::{
    io::{self, Error},
    str::FromStr,
};
use zenoh::prelude::sync::*;

mod driver;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// ros topic.
    #[arg(short = 't', long = "topic", default_value = "rt/imu")]
    topic: String,

    /// connect to Zenoh endpoint.
    #[arg(short = 'c', long = "connect", default_value = "tcp/127.0.0.1:7447")]
    connect: Vec<String>,

    /// list to Zenoh endpoint.
    #[arg(short = 'l', long = "listen")]
    listen: Vec<String>,

    /// zenoh connection mode.
    #[arg(short = 'm', long = "mode", default_value = "client")]
    mode: String,

    /// Specify the path to the spidevice.
    #[arg(short = 'd', long = "device", default_value = "/dev/spidev1.0")]
    spidevice: String,

    /// Specify the interrupt pin.
    #[arg(short = 'i', long = "interrupt", default_value = "IMU_INT")]
    hintn_pin: String,

    /// Specify the reset pin.
    #[arg(short = 'r', long = "reset", default_value = "IMU_RST")]
    reset_pin: String,

    /// Apply the Maivin2 FRS Configuration.
    #[arg(long = "configure")]
    configure: bool,
}

//#[async_std::main]
fn main() -> io::Result<()> {
    env_logger::init();
    let args = Args::parse();

    let mut config = Config::default();
    let mode = WhatAmI::from_str(&args.mode).unwrap();
    config.set_mode(Some(mode)).unwrap();
    config.connect.endpoints = args.connect.iter().map(|v| v.parse().unwrap()).collect();
    config.listen.endpoints = args.listen.iter().map(|v| v.parse().unwrap()).collect();
    let _ = config.scouting.multicast.set_enabled(Some(false));
    let _ = config.scouting.gossip.set_enabled(Some(false));
    let session = zenoh::open(config.clone()).res_sync().unwrap();
    info!("Opened Zenoh session");

    // Initializing the driver interface.
    debug!("Initializing driver wrapper with parameters:");
    debug!(
        "spidevice: {} hintn_pin: {} reset_pin: {}",
        args.spidevice, args.hintn_pin, args.reset_pin
    );

    let mut driver = driver::Driver::new(&args.spidevice, &args.hintn_pin, &args.reset_pin);
    driver.imu_driver.init().unwrap();
    if args.configure {
        if driver.configure_frs() {
            eprintln!("FRS records updated");
        } else {
            eprintln!("ERROR: FRS records not updated");
        }
        return Ok(());
    }

    info!("IMU Device Initialized");

    let report_update_cb =
        move |imu_driver: &BNO08x<SpiInterface<SpiDevice, GpiodIn, GpiodOut>>| {
            let [qi, qj, qk, qr] = imu_driver.rotation_quaternion().unwrap();
            let [lin_ax, lin_ay, lin_az] = imu_driver.accelerometer().unwrap();
            let [ang_ax, ang_ay, ang_az] = imu_driver.gyro().unwrap();

            trace!("Pose:   x: {}, y: {}, z: {}, w: {}", qi, qj, qk, qr);
            trace!(
                "Accel:  x: {}, y: {}, z: {} [m/s^2]",
                lin_ax,
                lin_ay,
                lin_az
            );
            trace!(
                "Gryo:   x: {}, y: {}, z: {} [rad/s] \n",
                ang_ax,
                ang_ay,
                ang_az
            );

            let msg = sensor_msgs::IMU {
                header: std_msgs::Header {
                    stamp: timestamp().unwrap(),
                    frame_id: "".to_owned(),
                },
                orientation: geometry_msgs::Quaternion {
                    x: qi as f64,
                    y: qj as f64,
                    z: qk as f64,
                    w: qr as f64,
                },
                orientation_covariance: [-1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                angular_velocity: geometry_msgs::Vector3 {
                    x: ang_ax as f64,
                    y: ang_ay as f64,
                    z: ang_az as f64,
                },
                angular_velocity_covariance: [-1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                linear_acceleration: geometry_msgs::Vector3 {
                    x: lin_ax as f64,
                    y: lin_ay as f64,
                    z: lin_az as f64,
                },
                linear_acceleration_covariance: [-1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            };

            let encoded = cdr::serialize::<_, _, CdrLe>(&msg, Infinite).unwrap();
            session
                .put(&args.topic, encoded)
                .encoding(Encoding::WithSuffix(
                    KnownEncoding::AppOctetStream,
                    "sensor_msgs/msg/Imu".into(),
                ))
                .res()
                .unwrap();
            trace!("Message sent on topic {}", args.topic);
        };

    driver.enable_reports();

    driver.imu_driver.add_sensor_report_callback(
        SENSOR_REPORTID_ROTATION_VECTOR,
        String::from("report_update_cb"),
        report_update_cb,
    );

    loop {
        let _msg_count = driver.imu_driver.handle_messages(5, 10);
        delay_ms(5);
    }
}

fn timestamp() -> Result<builtin_interfaces::Time, Error> {
    let mut tp = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    let err = unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC_RAW, &mut tp) };
    if err != 0 {
        return Err(Error::last_os_error());
    }

    Ok(builtin_interfaces::Time {
        sec: tp.tv_sec as i32,
        nanosec: tp.tv_nsec as u32,
    })
}
