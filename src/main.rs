// Copyright 2025 Au-Zone Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

mod args;
mod driver;

use args::Args;
use bno08x_rs::{
    interface::{
        gpio::{GpiodIn, GpiodOut},
        spidev::SpiDevice,
        SpiInterface,
    },
    BNO08x, SENSOR_REPORTID_ROTATION_VECTOR,
};
use clap::Parser;
use driver::Driver;
use edgefirst_schemas::{builtin_interfaces, geometry_msgs, sensor_msgs, serde_cdr, std_msgs};
use log::{debug, error, info, trace};
use std::{
    io::Error,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tracing::info_span;
use tracing_subscriber::{layer::SubscriberExt as _, Layer as _, Registry};
use tracy_client::frame_mark;
use zenoh::{
    bytes::{Encoding, ZBytes},
    Session, Wait,
};

const SUCCESS_TIME_LIMIT: Duration = Duration::from_secs(3);

fn main() {
    let args = Args::parse();
    if args.configure {
        let mut driver = Driver::new(&args.device, &args.interrupt, &args.reset);
        driver.imu_driver.init().unwrap();
        match driver.configure_frs() {
            Ok(_) => info!("FRS records updated"),
            Err(e) => error!("ERROR: FRS records not updated: {}", e),
        }
        return;
    }

    args.tracy.then(tracy_client::Client::start);

    let stdout_log = tracing_subscriber::fmt::layer()
        .pretty()
        .with_filter(args.rust_log);

    let journald = match tracing_journald::layer() {
        Ok(journald) => Some(journald.with_filter(args.rust_log)),
        Err(_) => None,
    };

    let tracy = match args.tracy {
        true => Some(tracing_tracy::TracyLayer::default().with_filter(args.rust_log)),
        false => None,
    };

    let subscriber = Registry::default()
        .with(stdout_log)
        .with(journald)
        .with(tracy);
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
    tracing_log::LogTracer::init().unwrap();

    let session = zenoh::open(args.clone()).wait().unwrap();

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

    error!(
        "{} Consecutive failures. Exiting...",
        consecutive_fail_count
    );
}

// This function will reset and initialize the IMU, enable reports, and send
// messages. If no message has been sent for while, the function will return.
// The function returns total elapsed duration
fn run_imu(args: &Args, session: Session) -> Duration {
    let fail_time_limit = Duration::from_millis(args.timeout);
    // Initializing the driver interface.
    debug!("Initializing driver wrapper with parameters:");
    debug!(
        "device: {} interrupt: {} reset: {}",
        args.device, args.interrupt, args.reset
    );

    let mut driver = driver::Driver::new(&args.device, &args.interrupt, &args.reset);
    if let Err(e) = driver.imu_driver.init() {
        error!("Could not initialize driver: {:?}", e);
        return Duration::from_nanos(0);
    }
    if let Err(e) = driver.enable_reports() {
        error!("Could not initialize reports: {:?}", e);
        return Duration::from_nanos(0);
    }

    info!("IMU Device Initialized");

    let last_send = Arc::from(Mutex::from((Instant::now(), false)));
    let last_send_ = last_send.clone();
    let report_update_cb =
        move |imu_driver: &BNO08x<SpiInterface<SpiDevice, GpiodIn, GpiodOut>>| {
            info_span!("publish").in_scope(|| {
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

                let buf = ZBytes::from(serde_cdr::serialize(&msg).unwrap());
                let enc = Encoding::APPLICATION_CDR.with_schema("sensor_msgs/msg/Imu");

                session.put(&args.topic, buf).encoding(enc).wait().unwrap();
                let mut last_send_locked = last_send.lock().unwrap();
                *(last_send_locked) = (Instant::now(), true);
            });

            args.tracy.then(frame_mark);
        };

    driver.imu_driver.add_sensor_report_callback(
        SENSOR_REPORTID_ROTATION_VECTOR,
        String::from("report_update_cb"),
        report_update_cb,
    );
    let start = Instant::now();
    loop {
        let _msg_count = driver.imu_driver.handle_messages(2, 10);
        let lock = last_send_.lock().unwrap();
        let last_msg_time = lock.0;
        let started = lock.1;
        let elapsed = last_msg_time.elapsed();

        let time_limit = if started {
            fail_time_limit
        } else {
            // 5x higher time limit for reading the first message
            fail_time_limit * 5
        };

        if elapsed > time_limit {
            error!("Last message was sent {:?} ago. Resetting IMU...", elapsed);
            return start.elapsed();
        }
        // Don't need to sleep in this loop because handle_messages uses a sleep
        // for the message polling, so if there is no message the
        // handle_messages function will sleep the thread
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
