//! Provides IMU driver initializations. 

use bno08x::interface::{SpiInterface, 
                        spidev::SpiDevice, 
                        delay::DelayMs, 
                        gpio::{GpiodIn, GpiodOut}};
use bno08x::wrapper::BNO08x;

pub struct Driver {
    pub imu_driver: BNO08x<SpiInterface<SpiDevice, GpiodIn, GpiodOut>>
}

impl Driver {
    /// Creates a Driver struct object initializing the driver wrapper 
    /// with the path to the spidevice, gpiochip resources, and the  
    /// pins set for spi communications. 
    pub fn new(
        spidevice: &str, 
        gpiochip: &str, 
        hintn_pin: u32, 
        reset_pin: u32) -> Self {
        
        let imu_driver = match BNO08x::new_bno08x(
                spidevice, gpiochip, hintn_pin, reset_pin) {
            Ok(imu_driver) => imu_driver,
            Err(_) => panic!("Initializing IMU driver failed!"),
        };        
        Self { 
            imu_driver
        }
    }

    /// Settings to set for the driver that was initialized. 
    pub fn initialize_driver(&mut self, delay_source: &mut impl DelayMs) {
        self.imu_driver.init(delay_source).unwrap();
        self.imu_driver.enable_rotation_vector(50).unwrap();
    }
}