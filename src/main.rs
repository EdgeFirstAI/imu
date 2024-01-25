#![crate_name = "camerapose"]

mod computations;
mod driver;
mod server;

use crate::{driver::Driver, server::Server};

use bno08x::{
    interface::{
        delay::delay_ms,
        gpio::{GpiodIn, GpiodOut},
        spidev::SpiDevice,
        SpiInterface,
    },
    wrapper::{BNO08x, SENSOR_REPORTID_ROTATION_VECTOR},
};
use computations::computations::{quaternion2euler, rad2degrees};
use std::io::{self};
use structopt::StructOpt;

use chrono::{offset::Utc, DateTime};
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
        short = "h",
        long = "hintn_pin",
        help = "Specify the interrupt pin",
        default_value = "IMU_INT"
    )]
    hintn_pin: String,

    #[structopt(
        short = "r",
        long = "reset_pin",
        help = "Specify the reset pin",
        default_value = "IMU_RST"
    )]
    reset_pin: String,

    #[structopt(
        short = "c",
        long = "configure",
        help = "Apply ADIS2 FRS configuration"
    )]
    configure: bool,

    #[structopt(long = "verbose", help = "Enables verbose output")]
    verbose: bool,
}
fn main() -> io::Result<()> {
    let opt = Opt::from_args();

    macro_rules! log {
        ($( $args:expr ),*) => { if opt.verbose {println!( $( $args ),* );} }
    }

    // Initializing the driver interface.
    log!("[INFO] Initializing driver wrapper with parameters:");
    log!(
        "* spidevice: {}\n* hintn_pin: {}\n* reset_pin: {}",
        opt.spidevice,
        opt.hintn_pin,
        opt.reset_pin
    );
    let mut driver = Driver::new(&opt.spidevice, &opt.hintn_pin, &opt.reset_pin);
    driver.imu_driver.init().unwrap();
    if opt.configure {
        if driver.configure_frs() {
            log!("FRS records updated");
            return Ok(());
        }
        eprintln!("ERROR: FRS records not updated");
        return Err(std::io::Error::new(std::io::ErrorKind::Other, "FRS records not updated"));
    }
    // Starting and initializing the server.
    println!("[INFO] Starting server at endpoint: {}", opt.endpoint);
    let server = Server::new(opt.endpoint);
    server.start_server();

    let report_update_cb = move |imu_driver: &BNO08x<
        SpiInterface<SpiDevice, GpiodIn, GpiodOut>,
    >| {
        let [qi, qj, qk, qr] = imu_driver.rotation_quaternion().unwrap();
        let rotation_update_time = imu_driver.report_update_time(SENSOR_REPORTID_ROTATION_VECTOR);
        let attitude = quaternion2euler(qr, qi, qj, qk).map(rad2degrees);
        let accelerometer = imu_driver.accelerometer().unwrap();
        let gyroscope = imu_driver.gyro().unwrap();
        let magnetometer = imu_driver.mag_field().unwrap();
        let rot_acc = rad2degrees(imu_driver.rotation_acc());
        let msg = server.send_message(
            attitude,
            accelerometer,
            gyroscope,
            magnetometer,
            rot_acc,
            rotation_update_time,
        );
        let system_time = SystemTime::now();
        let datetime: DateTime<Utc> = system_time.into();
        log!(
            "Message sent at {}:\n{}",
            datetime.format("%d/%m/%Y %T"),
            msg
        );
    };

    driver.enable_reports();

    driver.imu_driver.add_sensor_report_callback(
        SENSOR_REPORTID_ROTATION_VECTOR,
        String::from("report_update_cb"),
        report_update_cb,
    );

    // Starting the loop process.
    let system_time = SystemTime::now();
    let datetime: DateTime<Utc> = system_time.into();
    log!("{}", datetime.format("%d/%m/%Y %T"));

    println!("[INFO] Reading IMU and pushing messages...");
    let loop_interval = 5;

    loop {
        let _msg_count = driver.imu_driver.handle_messages(5, 10);
        delay_ms(loop_interval);
    }
}
