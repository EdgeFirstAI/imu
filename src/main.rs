#![crate_name = "camerapose"]

//! IMU Application Server
//! Binds PUB socket to tcp://*:5556 
//! Publishes random yaw, pitch, roll values.

use rand::Rng;
use std::f64::consts::PI;

// Source can be found in: https://github.com/erickt/rust-zmq/blob/master/examples/zguide/helloworld_server/main.rs

fn main() {
    let context = zmq::Context::new();
    let publisher = context.socket(zmq::PUB).unwrap();

    assert!(publisher.bind("tcp://*:5556").is_ok());
    let mut rng = rand::thread_rng();

    loop {
        //let zipcode = rng.gen_range(0..100_000);
        let yaw = rng.gen_range(-2.0*PI..2.0*PI);
        let pitch = rng.gen_range(-2.0*PI..2.0*PI);
        let roll = rng.gen_range(-2.0*PI..2.0*PI);

        // this is slower than C because the current format! implementation is
        // very, very slow. Several orders of magnitude slower than glibc's
        // sprintf
        let update = format!("{:.2} {:.2} {:.2}", yaw, pitch, roll);
        publisher.send(&update, 0).unwrap();
    }

    // note: destructors mean no explicit cleanup necessary
}

