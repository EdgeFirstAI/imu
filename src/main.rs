#![crate_name = "camerapose"]

mod server;
mod driver;
mod computations;

use crate::driver::Driver;
use crate::server::Server;
use computations::computations::{quaternion2euler, rad2degrees};
use bno08x::interface::delay::{TimerMs, DelayMs};
use structopt::StructOpt;
use std::io::{self};

use chrono::offset::Utc;
use chrono::DateTime;
use std::time::SystemTime;

#[derive(StructOpt, Debug)]
#[structopt(
    name = "IMU Application",
    about = "Pushes IMU angles to the endpoint set."
)]
struct Opt {
    #[structopt(
        short = "e",
        long = "endpoint",
        help = "Set the endpoint to push data",
        default_value = "ipc:///tmp/pose.pub"
    )]
    endpoint: String,

    #[structopt(
        short = "s",
        long = "spidevice",
        help = "Set the path to the spidevice",
        default_value = "/dev/spidev1.0"
    )]
    spidevice: String,

    #[structopt(
        short = "g",
        long = "gpiochip",
        help = "Set the path to the gpiochip",
        default_value = "/dev/gpiochip5"
    )]
    gpiochip: String,

    #[structopt(
        short = "h",
        long = "hintn_pin",
        help = "Specify the _ pin",
        default_value = "2"
    )]
    hintn_pin: u32,

    #[structopt(
        short = "r",
        long = "reset_pin",
        help = "Specify the reset pin",
        default_value = "0"
    )]
    reset_pin: u32,
}

fn main() -> io::Result<()> {
    let opt = Opt::from_args();

    // Starting and initializing the server.
    println!("[INFO] Starting server at endpoint: {}", opt.endpoint);
    let server = Server::new(opt.endpoint); 
    server.start_server();

    // Initializing the driver interface.
    println!("[INFO] Initializing driver wrapper with parameters:");
    println!("* spidevice: {}\n* gpiochip: {}\n* hintn_pin: {}\n* reset_pin: {}",
            opt.spidevice, opt.gpiochip, opt.hintn_pin, opt.reset_pin);
    let mut delay_source = TimerMs {};
    let mut driver = Driver::new(
        &opt.spidevice,
        &opt.gpiochip,
        opt.hintn_pin,
        opt.reset_pin
    ); 
    driver.initialize_driver(&mut delay_source);

    // Starting the loop process.
    let system_time = SystemTime::now();
    let datetime: DateTime<Utc> = system_time.into();
    println!("{}", datetime.format("%d/%m/%Y %T"));

    println!("[INFO] Reading IMU and pushing messages...");
    let loop_interval = 50;
    loop {
        let _msg_count = driver
                            .imu_driver
                            .handle_messages(&mut delay_source, 10, 10);
        delay_source.delay_ms(loop_interval);
        let [qi, qj, qk, qr] = driver.imu_driver.rotation_quaternion().unwrap();
        let attitude = rad2degrees(quaternion2euler(qr, qi, qj, qk));
        let accelerometer = driver.imu_driver.accelerometer().unwrap();
        let gyroscope = driver.imu_driver.gyro().unwrap();
        let magnetometer = driver.imu_driver.mag_field().unwrap();
        let rot_acc: f32 = driver.imu_driver.rotation_acc();
        server.send_message(attitude, accelerometer, gyroscope, magnetometer, rot_acc);
        
        let system_time = SystemTime::now();
        let datetime: DateTime<Utc> = system_time.into();
        print!("\r{}", datetime.format("%d/%m/%Y %T"));
    }
}
