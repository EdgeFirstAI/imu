// Copyright 2025 Au-Zone Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

use bno08x_rs::{
    SENSOR_REPORTID_ACCELEROMETER, SENSOR_REPORTID_GYROSCOPE, SENSOR_REPORTID_ROTATION_VECTOR,
};
use clap::{Parser, ValueEnum};
use serde_json::json;
use tracing::level_filters::LevelFilter;
use zenoh::config::{Config, WhatAmI};

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum PrimarySensor {
    RotationVector,
    Accelerometer,
    Gyroscope,
}

impl PrimarySensor {
    pub fn to_sensor_id(self) -> u8 {
        match self {
            PrimarySensor::RotationVector => SENSOR_REPORTID_ROTATION_VECTOR,
            PrimarySensor::Accelerometer => SENSOR_REPORTID_ACCELEROMETER,
            PrimarySensor::Gyroscope => SENSOR_REPORTID_GYROSCOPE,
        }
    }
}
/// Command-line arguments for EdgeFirst IMU Node.
///
/// This structure defines all configuration options for the IMU node,
/// including device paths, Zenoh configuration, and debugging options.
/// Arguments can be specified via command line or environment variables.
///
/// # Example
///
/// ```bash
/// # Via command line
/// edgefirst-imu --timeout 200 --topic rt/imu
///
/// # Via environment variables
/// export TIMEOUT=200
/// export MODE=client
/// edgefirst-imu
/// ```
#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// IMU times out after not recieving a message for this many
    /// milliseconds,
    #[arg(long, env = "TIMEOUT", default_value = "165")]
    pub timeout: u64,

    /// Specify the path to the spidevice.
    #[arg(long, default_value = "/dev/spidev1.0")]
    pub device: String,

    /// Specify the interrupt pin.
    #[arg(long, default_value = "IMU_INT")]
    pub interrupt: String,

    /// Specify the reset pin.
    #[arg(long, default_value = "IMU_RST")]
    pub reset: String,

    /// The primary sensor is used to determine the timestamp for messages
    /// sent by the node, and is also used to determine Zenoh message frequency.
    #[arg(long, env, default_value = "rotation-vector")]
    pub primary_sensor: PrimarySensor,

    /// Requested update rate for the rotation vector in microseconds.
    /// The driver may not be able to meet these rates, but will try its best.
    /// Set to 0 to disable the rotation vector.
    #[arg(long, env, default_value = "5000")]
    pub update_rot_us: u32,

    /// Requested update rate for the accelerometer in microseconds.
    /// The driver may not be able to meet these rates, but will try its best.
    /// Set to 0 to disable the accelerometer.
    #[arg(long, env, default_value = "10000")]
    pub update_accel_us: u32,

    /// Requested update rate for the gyroscope in microseconds.
    /// The driver may not be able to meet these rates, but will try its best.
    /// Set to 0 to disable the gyroscope.
    #[arg(long, env, default_value = "10000")]
    pub update_gyro_us: u32,

    /// Apply the Maivin2 FRS Configuration.
    #[arg(long)]
    pub configure: bool,

    /// ros topic.
    #[arg(long, default_value = "rt/imu")]
    pub topic: String,

    /// Application log level
    #[arg(long, env = "RUST_LOG", default_value = "info")]
    pub rust_log: LevelFilter,

    /// Enable Tracy profiler broadcast
    #[arg(long, env = "TRACY")]
    pub tracy: bool,

    /// Zenoh participant mode (peer, client, or router)
    #[arg(long, env = "MODE", default_value = "peer")]
    mode: WhatAmI,

    /// Zenoh endpoints to connect to (can specify multiple)
    #[arg(long, env = "CONNECT")]
    connect: Vec<String>,

    /// Zenoh endpoints to listen on (can specify multiple)
    #[arg(long, env = "LISTEN")]
    listen: Vec<String>,

    /// Disable Zenoh multicast peer discovery
    #[arg(long, env = "NO_MULTICAST_SCOUTING")]
    no_multicast_scouting: bool,
}

impl From<Args> for Config {
    fn from(args: Args) -> Self {
        let mut config = Config::default();

        config
            .insert_json5("mode", &json!(args.mode).to_string())
            .unwrap();

        let connect: Vec<_> = args.connect.into_iter().filter(|s| !s.is_empty()).collect();
        if !connect.is_empty() {
            config
                .insert_json5("connect/endpoints", &json!(connect).to_string())
                .unwrap();
        }

        let listen: Vec<_> = args.listen.into_iter().filter(|s| !s.is_empty()).collect();
        if !listen.is_empty() {
            config
                .insert_json5("listen/endpoints", &json!(listen).to_string())
                .unwrap();
        }

        if args.no_multicast_scouting {
            config
                .insert_json5("scouting/multicast/enabled", &json!(false).to_string())
                .unwrap();
        }

        config
            .insert_json5("scouting/multicast/interface", &json!("lo").to_string())
            .unwrap();

        config
    }
}
