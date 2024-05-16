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
use env_logger::Env;
use log::{debug, error, info, trace};
use std::{
    io::{self, Error},
    str::FromStr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use zenoh::prelude::sync::*;

use crate::driver::Driver;

mod driver;

const SUCCESS_TIME_LIMIT: Duration = Duration::from_secs(3);

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

    /// IMU times out after not recieving a message for this many
    /// milliseconds,
    #[arg(short = 't', long = "timeout", default_value = "165")]
    imu_msg_timeout: u64,
}

fn main() -> io::Result<()> {
    env_logger::init_from_env(Env::default().default_filter_or("info"));
    let args = Args::parse();
    if args.configure {
        let mut driver = Driver::new(&args.spidevice, &args.hintn_pin, &args.reset_pin);
        driver.imu_driver.init().unwrap();
        match driver.configure_frs() {
            Ok(_) => info!("FRS records updated"),
            Err(e) => error!("ERROR: FRS records not updated: {}", e),
        }
        return Ok(());
    }
    let mut config = Config::default();
    let mode = WhatAmI::from_str(&args.mode).unwrap();
    config.set_mode(Some(mode)).unwrap();
    config.connect.endpoints = args.connect.iter().map(|v| v.parse().unwrap()).collect();
    config.listen.endpoints = args.listen.iter().map(|v| v.parse().unwrap()).collect();
    let _ = config.scouting.multicast.set_enabled(Some(false));
    let _ = config.scouting.gossip.set_enabled(Some(false));
    let session = zenoh::open(config.clone()).res_sync().unwrap().into_arc();
    info!("Opened Zenoh session");

    let mut consecutive_fail_count = 0;
    while consecutive_fail_count < 3 {
        let elapsed = run_imu(&args, session.clone());
        // considered a success if the IMU runs for more than the time limit
        if elapsed > SUCCESS_TIME_LIMIT {
            consecutive_fail_count = 0;
        } else {
            consecutive_fail_count += 1;
        }
    }
    eprintln!(
        "[ERROR] {} Consecutive failures. Exiting...",
        consecutive_fail_count
    );
    Ok(())
}

// This function will reset and initialize the IMU, enable reports, and send
// messages. If no message has been sent for while, the function will return.
// The function returns total elapsed duration
fn run_imu(args: &Args, session: Arc<Session>) -> Duration {
    let fail_time_limit = Duration::from_millis(args.imu_msg_timeout);
    // Initializing the driver interface.
    debug!("Initializing driver wrapper with parameters:");
    debug!(
        "spidevice: {} hintn_pin: {} reset_pin: {}",
        args.spidevice, args.hintn_pin, args.reset_pin
    );

    let mut driver = driver::Driver::new(&args.spidevice, &args.hintn_pin, &args.reset_pin);
    if let Err(e) = driver.imu_driver.init() {
        error!("Could not initialize driver: {:?}", e);
        return Duration::from_nanos(0);
    }
    if let Err(e) = driver.enable_reports() {
        error!("Could not initialize reports: {:?}", e);
        return Duration::from_nanos(0);
    }

    info!("IMU Device Initialized");

    let last_send = Arc::from(Mutex::from(Instant::now()));
    let last_send_ = last_send.clone();
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
            let mut last_send_locked = last_send.lock().unwrap();
            *(last_send_locked) = Instant::now();
            trace!("Message sent on topic {}", args.topic);
        };

    driver.imu_driver.add_sensor_report_callback(
        SENSOR_REPORTID_ROTATION_VECTOR,
        String::from("report_update_cb"),
        report_update_cb,
    );
    let start = Instant::now();
    let loop_interval = 2;

    loop {
        let _msg_count = driver.imu_driver.handle_messages(2, 10);
        let elapsed = last_send_.lock().unwrap().elapsed();

        if elapsed > fail_time_limit {
            println!(
                "[ERROR] Last message was sent {:?} ago. Resetting IMU...",
                elapsed
            );
            return start.elapsed();
        }
        delay_ms(loop_interval);
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
