// Copyright 2025 Au-Zone Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

//! Provides IMU driver initializations.

use bno08x_rs::{
    interface::{
        delay::delay_ms,
        gpio::{GpiodIn, GpiodOut},
        spidev::SpiDevice,
        SpiInterface,
    },
    BNO08x, SENSOR_REPORTID_ACCELEROMETER, SENSOR_REPORTID_GYROSCOPE,
    SENSOR_REPORTID_ROTATION_VECTOR,
};

use crate::args::Args;

pub struct Driver<'a> {
    pub imu_driver: BNO08x<'a, SpiInterface<SpiDevice, GpiodIn, GpiodOut>>,
}

impl Driver<'_> {
    /// Creates a Driver struct object initializing the driver wrapper
    /// with the path to the spidevice, gpiochip resources, and the
    /// pins set for spi communications.
    pub fn new(spidevice: &str, hintn_pin: &str, reset_pin: &str) -> Self {
        let imu_driver = match BNO08x::new_spi_from_symbol(spidevice, hintn_pin, reset_pin) {
            Ok(imu_driver) => imu_driver,
            Err(_) => panic!("Initializing IMU driver failed!"),
        };
        Self { imu_driver }
    }

    /// Settings to set for the driver that was initialized.
    pub fn enable_reports(&mut self, args: &Args) -> Result<(), String> {
        let reports = [
            (SENSOR_REPORTID_ROTATION_VECTOR, args.update_rot_us),
            (SENSOR_REPORTID_ACCELEROMETER, args.update_accel_us),
            (SENSOR_REPORTID_GYROSCOPE, args.update_gyro_us),
        ];

        let max_tries = 5;

        for (r, t) in reports {
            if t == 0 {
                continue;
            }

            let mut i = 0;
            while i < max_tries && !self.imu_driver.is_report_enabled(r) {
                let _ = self.imu_driver.enable_report_us(r, t);
                i += 1;
            }

            if !self.imu_driver.is_report_enabled(r) {
                return Err(format!("Could not enable report {}", r));
            }

            delay_ms(100);
        }
        Ok(())
    }

    pub fn configure_frs(&mut self) -> Result<(), String> {
        // Need to enable a report so that the IMU reports back to the program.
        // Writes don't seem to work if the IMU doesn't also have anything send
        // This is because the PS0/WAKE pin isn't connected to GPIO so we can only write
        // to the IMU when it's awake, which is when it's sending reports back
        // https://au-zone.atlassian.net/browse/TOP2-188
        // MVN2-300000 R00A GNSS-IMU Schematics.PDF
        let max_tries = 5;

        let report_id = SENSOR_REPORTID_ACCELEROMETER;
        let mut i = 0;
        while i < max_tries && !self.imu_driver.is_report_enabled(report_id) {
            let _ = self.imu_driver.enable_report(report_id, 100);
            i += 1;
        }
        if !self.imu_driver.is_report_enabled(report_id) {
            return Err(format!(
                "Did not enable report {} for communication",
                report_id
            ));
        }
        delay_ms(1000);

        let mut last_err = "Success".to_string();
        for _ in 0..max_tries {
            // The driver will normalize the quaternion so we don't need to normalize it
            // ourselves
            match self
                .imu_driver
                .set_sensor_orientation(-1.0, 0.0, 0.0, 1.0, 2000)
            {
                Ok(v) if v => return Ok(()),
                Ok(_) => last_err = "FRS records write failed".to_string(),
                Err(e) => last_err = format!("{:?}", e),
            }
        }
        Err(format!(
            "Did not update sensor orientation FRS records after {} tries. The last error was {}",
            max_tries, last_err,
        ))
    }
}
