//! Provides IMU driver initializations.

use bno08x::{
    interface::{
        delay::delay_ms,
        gpio::{GpiodIn, GpiodOut},
        spidev::SpiDevice,
        SpiInterface,
    },
    wrapper::{
        BNO08x, SENSOR_REPORTID_ACCELEROMETER, SENSOR_REPORTID_GYROSCOPE,
        SENSOR_REPORTID_MAGNETIC_FIELD, SENSOR_REPORTID_ROTATION_VECTOR,
    },
};

pub struct Driver<'a> {
    pub imu_driver: BNO08x<'a, SpiInterface<SpiDevice, GpiodIn, GpiodOut>>,
}

pub const ROTATION_VECTOR_UPDATE_MS: u16 = 33;
pub const ACCELEROMETER_UPDATE_MS: u16 = 100;
pub const GYROSCOPE_UPDATE_MS: u16 = 100;
pub const MAGNETIC_FIELD_UPDATE_MS: u16 = 300;

impl Driver<'_> {
    /// Creates a Driver struct object initializing the driver wrapper
    /// with the path to the spidevice, gpiochip resources, and the  
    /// pins set for spi communications.
    pub fn new(spidevice: &str, hintn_pin: &str, reset_pin: &str) -> Self {
        let imu_driver = match BNO08x::new_bno08x_from_symbol(spidevice, hintn_pin, reset_pin) {
            Ok(imu_driver) => imu_driver,
            Err(_) => panic!("Initializing IMU driver failed!"),
        };
        Self { imu_driver }
    }

    /// Settings to set for the driver that was initialized.
    pub fn enable_reports(&mut self) -> Result<(), String> {
        let reports = [
            (SENSOR_REPORTID_ROTATION_VECTOR, ROTATION_VECTOR_UPDATE_MS),
            (SENSOR_REPORTID_ACCELEROMETER, ACCELEROMETER_UPDATE_MS),
            (SENSOR_REPORTID_GYROSCOPE, GYROSCOPE_UPDATE_MS),
            (SENSOR_REPORTID_MAGNETIC_FIELD, MAGNETIC_FIELD_UPDATE_MS),
        ];

        let max_tries = 5;

        for (r, t) in reports {
            let mut i = 0;
            while i < max_tries && !self.imu_driver.is_report_enabled(r) {
                let _ = self.imu_driver.enable_report(r, t);
                i += 1;
            }

            if !self.imu_driver.is_report_enabled(r) {
                return Err(format!("Could not enable report {}", r));
            }

            delay_ms(100);
        }
        Ok(())
    }

    pub fn configure_frs(&mut self) -> bool {
        // Need to enable a report so that the IMU reports back to the program.
        // Writes don't seem to work if the IMU doesn't also have anything send
        let max_tries = 5;

        let report_id = SENSOR_REPORTID_ACCELEROMETER;
        let mut i = 0;
        while i < max_tries && !self.imu_driver.is_report_enabled(report_id) {
            let _ = self.imu_driver.enable_report(report_id, 100);
            i += 1;
        }
        if !self.imu_driver.is_report_enabled(report_id) {
            return false;
        }
        delay_ms(1000);

        let mut success = false;
        i = 0;
        while i < max_tries && !success {
            // The driver will normalize the quaternion so we don't need to normalize it
            // ourselves
            match self
                .imu_driver
                .set_sensor_orientation(-1.0, 0.0, 0.0, 1.0, 2000)
            {
                Ok(v) => success = v,
                Err(_) => success = false,
            };
        }

        return success;
    }
}
