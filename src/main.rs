use zenoh_ros_type::common_interfaces::sensor_msgs::IMU;
use zenoh::publication::CongestionControl;
use zenoh::prelude::sync::*;
use cdr::{CdrLe, Infinite};
use std::io::{self};
use clap::Parser;

mod connection;
mod messages;
mod driver;

use bno08x::{
    interface::{
        delay::delay_ms,
        gpio::{GpiodIn, GpiodOut},
        spidev::SpiDevice,
        SpiInterface,
    },
    wrapper::{BNO08x, SENSOR_REPORTID_ROTATION_VECTOR},
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// zenoh connection mode.
    #[arg(short='m', long="mode", default_value = "peer")]
    mode: String,

    /// connect to endpoint.
    #[arg(short='e', long="endpoint")]
    endpoint: Vec<String>,

    /// ros topic.
    #[arg(short='t', long="topic", default_value = "rt/imu")]
    topic: String,

    /// publisher mode (default is subscriber mode).
    #[arg(short='p', long="publisher")]
    publisher: bool,

    /// Specify the path to the spidevice.
    #[arg(short='d', long="device", default_value = "/dev/spidev1.0")]
    spidevice: String,

    /// Specify the interrupt pin.
    #[arg(short='i', long="interrupt", default_value = "IMU_INT")]
    hintn_pin: String,

    /// Specify the reset pin.
    #[arg(short='r', long="reset", default_value = "IMU_RST")]
    reset_pin: String,

    /// Apply the ADIS2 FRS Configuration.
    #[arg(short='c', long="configure")]
    configure: bool,

    /// Enable the verbose output.
    #[arg(short='v', long="verbose")]
    verbose: bool,
}

//#[async_std::main]
fn main() -> io::Result<()> {
    let args = Args::parse();
    
    // Start a Zenoh connection at the endpoint.
    let session = connection::start_session(&args.mode, &args.endpoint).unwrap();

    // Publish messages.
    if args.publisher {
        macro_rules! log {
            ($( $args:expr ),*) => { if args.verbose {println!( $( $args ),* );} }
        }
    
        // Initializing the driver interface.
        log!("[INFO] Initializing driver wrapper with parameters:");
        log!(
            "* spidevice: {}\n* hintn_pin: {}\n* reset_pin: {}",
            args.spidevice,
            args.hintn_pin,
            args.reset_pin
        );
    
        let mut driver = driver::Driver::new(&args.spidevice, &args.hintn_pin, &args.reset_pin);
        driver.imu_driver.init().unwrap();
        if args.configure {
            if driver.configure_frs() {
                log!("FRS records updated");
            } else {
                log!("ERROR: FRS records not updated");
            }
            return Ok(());
        }
    
        let report_update_cb = move |imu_driver: &BNO08x<
            SpiInterface<SpiDevice, GpiodIn, GpiodOut>,
        >| {
            let publisher = session
                .declare_publisher(&args.topic)
                .congestion_control(CongestionControl::Block)
                .res()
                .unwrap();

            let [qi, qj, qk, qr] = imu_driver.rotation_quaternion().unwrap();
            imu_driver.report_update_time(SENSOR_REPORTID_ROTATION_VECTOR);
            let [lin_ax, lin_ay, lin_az] = imu_driver.accelerometer().unwrap();
            let [ang_ax, ang_ay, ang_az] = imu_driver.gyro().unwrap();

           
            let frame = String::from("ImuMap");
            println!("Publish IMU on '{}' for '{}')...", &args.topic, frame);
            // Build the IMU message type.
            let header = messages:: header(&frame);
            let orientation = messages::orientation(qi as f64,qj as f64,qk as f64, qr as f64);
            let linear_acceleration = messages::linear_acceleration(lin_ax as f64, lin_ay as f64, lin_az as f64);
            let angular_velocity = messages::angular_velocity(ang_ax as f64, ang_ay as f64, ang_az as f64);
            let imu = messages::imu_message(
                header, 
                orientation, 
                [-1.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0],
                angular_velocity,
                [-1.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0],
                linear_acceleration,
                [-1.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0],
            );

            let encoded = cdr::serialize::<_, _, CdrLe>(&imu, Infinite).unwrap();
            publisher.put(encoded).res().unwrap();
            
        };
    
        driver.enable_reports();

        driver.imu_driver.add_sensor_report_callback(
            SENSOR_REPORTID_ROTATION_VECTOR,
            String::from("report_update_cb"),
            report_update_cb,
        );

        loop {
            let _msg_count = driver.imu_driver.handle_messages(5, 10);
            delay_ms(5);
        }
    } 
    else {
        let subscriber = session.declare_subscriber(&args.topic).res().unwrap();
        while let Ok(sample) = subscriber.recv() {
            let decoded = cdr::deserialize_from::<_, IMU, _>(sample.value.payload.reader(), Infinite).unwrap();
            println!("Orientation {}, {}, {}, {}", decoded.orientation.x, decoded.orientation.y, decoded.orientation.z, decoded.orientation.w);
        }
        return Ok(());
    }
}
