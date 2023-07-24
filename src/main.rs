#![crate_name = "camerapose"]
mod server;
mod driver;

mod computations;

use bno08x::interface::delay::{TimerMs, DelayMs};
use crate::driver::Driver;
use computations::computations::quaternion2euler;
use crate::server::Server;
use structopt::StructOpt;

use std::io::{self};

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
    let server = Server::new(opt.endpoint); 
    server.start_server();

    // Initializing the driver interface.
    let mut delay_source = TimerMs {};
    let mut driver = Driver::new(
        &opt.spidevice,
        &opt.gpiochip,
        opt.hintn_pin,
        opt.reset_pin
    ); 
    driver.initialize_driver(&mut delay_source);

    // Starting the loop process. 
    let loop_interval = 50 as u8;
    println!("loop_interval: {}", loop_interval);

    loop {
        let _msg_count = driver.imu_driver.handle_all_messages(&mut delay_source, 10u8);
        delay_source.delay_ms(loop_interval);
        // println!("Current rotation: {:?}", imu_driver.rotation_quaternion());
        let [qr, qi, qj, qk] = driver.imu_driver.rotation_quaternion().unwrap();

        let (yaw, pitch, roll) = quaternion2euler(qr, qi, qj, qk);
        println!(
            "Current rotation: {}, {}, {}", yaw, pitch, roll 
        );

        // this is slower than C because the current format! implementation is
        // very, very slow. Several orders of magnitude slower than glibc's
        // sprintf
        let update = format!("{:.2} {:.2} {:.2}", yaw, pitch, roll);
        server.publisher.send(&update, 0).unwrap();
    }
}
