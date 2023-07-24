#![crate_name = "camerapose"]
mod server;

use crate::server::Server;
use structopt::StructOpt;
use bno08x::wrapper::BNO08x;

use std::io::{self};

#[derive(StructOpt, Debug)]
#[structopt(name = "IMU Application", about = "Pushes IMU angles to the endpoint set.")]
struct Opt {
    #[structopt(short = "e", long = "endpoint", help = "Set the endpoint to push data", default_value = "ipc:///tmp/pose.pub")]
    endpoint: String,
}

fn main() -> io::Result<()> {
    let opt = Opt::from_args();

    let mut imu_driver =
        BNO08x::new_bno08x("/dev/spidev1.0", "/dev/gpiochip5", 2, 0)?;

    let mut delay_source = TimerMs {};
    imu_driver.init(&mut delay_source).unwrap();
    imu_driver.enable_rotation_vector(50).unwrap();

    let loop_interval = 50 as u8;
    println!("loop_interval: {}", loop_interval);

    loop {
        let _msg_count =
            imu_driver.handle_all_messages(&mut delay_source, 10u8);
        // if _msg_count > 0 {
        //     println!("> {}", _msg_count);
        // }
        delay_source.delay_ms(loop_interval);
        // println!("Current rotation: {:?}", imu_driver.rotation_quaternion());
        let [qr, qi, qj, qk] = imu_driver.rotation_quaternion().unwrap();
        println!(
            "Current rotation: {:?}",
            quaternion_to_euler(qr, qi, qj, qk)
        );
    }

    let server = Server {
        endpoint: opt.endpoint,
    };
    server.start_server();
}