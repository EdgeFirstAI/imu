#![crate_name = "camerapose"]

mod computations;
mod driver;
mod server;

use crate::{
    driver::{Driver, ROTATION_VECTOR_UPDATE_MS},
    server::Server,
};

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
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
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

    #[structopt(
        short = "t",
        long = "timeout",
        help = "IMU times out after not recieving a message for this many milliseconds",
        default_value = "165"
    )]
    imu_msg_timeout: u64,

    #[structopt(long = "verbose", help = "Enables verbose output")]
    verbose: bool,
}

const SUCCESS_TIME_LIMIT: Duration = Duration::from_millis(ROTATION_VECTOR_UPDATE_MS as u64 * 100); // 3.3 s
fn main() {
    let opt = Opt::from_args();
    if opt.configure {
        let mut driver = Driver::new(&opt.spidevice, &opt.hintn_pin, &opt.reset_pin);
        driver.imu_driver.init().unwrap();
        if driver.configure_frs() {
            println!("FRS records updated");
        } else {
            eprintln!("ERROR: FRS records not updated");
        }
        return;
    }
    // Starting and initializing the server.
    println!("[INFO] Starting server at endpoint: {}", opt.endpoint);
    let server = Server::new(opt.endpoint.clone());
    server.start_server();
    let mut consecutive_fail_count = 0;
    while consecutive_fail_count < 3 {
        let elapsed = run_imu(&opt, &server);
        // considered a success if the IMU runs for more than the time limit
        if elapsed > SUCCESS_TIME_LIMIT {
            consecutive_fail_count = 0;
        } else {
            consecutive_fail_count += 1;
        }
    }
    eprintln!(
        "[ERROR] {} Consecutive failures. Exiting...",
        consecutive_fail_count
    );
}

fn run_imu(opt: &Opt, server: &Server) -> Duration {
    macro_rules! log {
        ($( $args:expr ),*) => { if opt.verbose {println!( $( $args ),* );} }
    }
    let start = Instant::now();

    let fail_time_limit = Duration::from_millis(opt.imu_msg_timeout);
    // Initializing the driver interface.
    log!("[INFO] Initializing driver wrapper with parameters:");
    log!(
        "* spidevice: {}\n* hintn_pin: {}\n* reset_pin: {}",
        opt.spidevice,
        opt.hintn_pin,
        opt.reset_pin
    );
    let mut driver = Driver::new(&opt.spidevice, &opt.hintn_pin, &opt.reset_pin);
    if let Err(e) = driver.imu_driver.init() {
        eprintln!("[ERROR] Could not initialize driver: {:?}", e);
        return start.elapsed();
    }
    if let Err(e) = driver.enable_reports() {
        eprintln!("[ERROR] Could not initialize reports: {:?}", e);
        return start.elapsed();
    }

    let last_send = Arc::from(Mutex::from(Instant::now()));
    let last_send_ = last_send.clone();
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
        let mut last_send_locked = last_send.lock().unwrap();
        *(last_send_locked) = Instant::now();
        let datetime: DateTime<Utc> = system_time.into();
        log!(
            "Message sent at {}:\n{}",
            datetime.format("%d/%m/%Y %T"),
            msg
        );
    };

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
    let loop_interval = 2;

    loop {
        let _msg_count = driver.imu_driver.handle_messages(2, 10);
        let elapsed = last_send_.lock().unwrap().elapsed();

        if elapsed > fail_time_limit {
            println!(
                "[ERROR] Last message was sent {:?} ago. Resetting IMU...",
                elapsed
            );
            return start.elapsed();
        }
        delay_ms(loop_interval);
    }
}
