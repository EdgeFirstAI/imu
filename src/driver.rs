
use bno08x::interface::gpio::{GpiodIn, GpiodOut};
use bno08x::interface::spidev::SpiDevice;
use bno08x::interface::SpiInterface;
use bno08x::{wrapper::BNO08x, interface::delay::DelayMs};

pub struct Driver {
    pub imu_driver: BNO08x<SpiInterface<SpiDevice, GpiodIn, GpiodOut>>
}

impl Driver {
    pub fn new(spidevice: &str, gpiochip: &str, hintn_pin: u32, reset_pin: u32) -> Self {
        let imu_driver = match BNO08x::new_bno08x(spidevice, gpiochip, hintn_pin, reset_pin) {
            Ok(imu_driver) => imu_driver,
            Err(_) => panic!("Initializing IMU driver failed!"),
        };        
        Self { 
            imu_driver
        }
    }

    pub fn initialize_driver(&mut self, delay_source: &mut impl DelayMs) {
        self.imu_driver.init(delay_source).unwrap();
        self.imu_driver.enable_rotation_vector(50).unwrap();
    }
}

