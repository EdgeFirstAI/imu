use cdr::{CdrLe, Infinite};
use clap::Parser;
use std::{io, str::FromStr, time::Instant};
use zenoh::prelude::sync::*;
mod connection;
mod driver;
mod messages;

use bno08x::{
    interface::{
        delay::delay_ms,
        gpio::{GpiodIn, GpiodOut},
        spidev::SpiDevice,
        SpiInterface,
    },
    wrapper::{BNO08x, SENSOR_REPORTID_ROTATION_VECTOR},
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// zenoh connection mode.
    #[arg(short = 'm', long = "mode", default_value = "peer")]
    mode: String,

    /// connect to Zenoh endpoint.
    #[arg(short = 'e', long = "endpoint")]
    endpoint: Vec<String>,

    /// ros topic.
    #[arg(short = 't', long = "topic", default_value = "rt/imu")]
    topic: String,

    /// Specify the path to the spidevice.
    #[arg(short = 'd', long = "device", default_value = "/dev/spidev1.0")]
    spidevice: String,

    /// Specify the interrupt pin.
    #[arg(short = 'i', long = "interrupt", default_value = "IMU_INT")]
    hintn_pin: String,

    /// Specify the reset pin.
    #[arg(short = 'r', long = "reset", default_value = "IMU_RST")]
    reset_pin: String,

    /// Apply the ADIS2 FRS Configuration.
    #[arg(short = 'c', long = "configure")]
    configure: bool,

    /// Enable the verbose output.
    #[arg(short = 'v', long = "verbose")]
    verbose: bool,
}

//#[async_std::main]
fn main() -> io::Result<()> {
    let start_time = Instant::now();

    let args = Args::parse();

    // Start a Zenoh connection at the endpoint.
    let mut config = Config::default();

    let mode = WhatAmI::from_str(&args.mode).unwrap();
    config.set_mode(Some(mode)).unwrap();
    // config.connect.endpoints = s.endpoints.iter().map(|v|
    // v.parse().unwrap()).collect();

    let session = zenoh::open(config).res_sync().unwrap();

    // Publish messages.
    macro_rules! log {
        ($( $args:expr ),*) => { if args.verbose {println!( $( $args ),* );} }
    }

    // Initializing the driver interface.
    log!("[INFO] Initializing driver wrapper with parameters:");
    log!(
        "* spidevice: {}\n* hintn_pin: {}\n* reset_pin: {}",
        args.spidevice,
        args.hintn_pin,
        args.reset_pin
    );

    let mut driver = driver::Driver::new(&args.spidevice, &args.hintn_pin, &args.reset_pin);
    driver.imu_driver.init().unwrap();
    if args.configure {
        if driver.configure_frs() {
            log!("FRS records updated");
        } else {
            log!("ERROR: FRS records not updated");
        }
        return Ok(());
    }
    // let publisher = session
    //     .declare_publisher(args.topic.clone())
    //     .congestion_control(CongestionControl::Block)
    //     .res()
    //     .unwrap();
    let frame = String::from("ImuMap");
    println!("Publish IMU on '{}' for '{}')...", &args.topic, frame);

    let report_update_cb =
        move |imu_driver: &BNO08x<SpiInterface<SpiDevice, GpiodIn, GpiodOut>>| {
            let [qi, qj, qk, qr] = imu_driver.rotation_quaternion().unwrap();
            let [lin_ax, lin_ay, lin_az] = imu_driver.accelerometer().unwrap();
            let [ang_ax, ang_ay, ang_az] = imu_driver.gyro().unwrap();

            log!("x: {}, y: {}, z: {}, w: {}", qi, qj, qk, qr);
            log!("x: {}, y: {}, z: {} [m/s^2]", lin_ax, lin_ay, lin_az);
            log!("x: {}, y: {}, z: {} [rad/s] \n", ang_ax, ang_ay, ang_az);

            // Build the IMU message type.
            let header = messages::header(&frame, start_time);
            let orientation = messages::orientation(qi as f64, qj as f64, qk as f64, qr as f64);
            let linear_acceleration =
                messages::linear_acceleration(lin_ax as f64, lin_ay as f64, lin_az as f64);
            let angular_velocity =
                messages::angular_velocity(ang_ax as f64, ang_ay as f64, ang_az as f64);
            let imu = messages::imu_message(
                header,
                orientation,
                [-1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                angular_velocity,
                [-1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                linear_acceleration,
                [-1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            );

            let encoded = cdr::serialize::<_, _, CdrLe>(&imu, Infinite).unwrap();
            session
                .put(&args.topic, encoded)
                .encoding(Encoding::WithSuffix(
                    KnownEncoding::AppOctetStream,
                    "sensor_msgs/msg/Imu".into(),
                ))
                .res()
                .unwrap();
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
