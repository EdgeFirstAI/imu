//! Crate to establish server intializations.
//! Source was found in https://github.com/erickt/rust-zmq/blob/master/examples/zguide/weather_server/main.rs
//! Publishes yaw, pitch, roll values gathered from 
//! the IMU to the dedicated endpoint set.
use zmq::{Context, Socket};

pub struct Server {
    pub endpoint: String,
    pub socket: Socket
}

impl Server {

    /// Creates a new struct object intializing the Socket.
    pub fn new(endpoint: String) -> Self {
        let context = Context::new();
        let socket = context.socket(zmq::PUB).unwrap();
        Self { 
            endpoint,
            socket
        }
    }

    /// Binds the socket with the dedicated endpoint set. 
    pub fn start_server(&self) {
        assert!(self.socket.bind(&self.endpoint).is_ok());
    }

    /// Sends the yaw, pitch, and roll values captured by the IMU.
    pub fn send_message(&self, yaw: f32, pitch: f32, roll: f32) {
        // this is slower than C because the current format! implementation is
        // very, very slow. Several orders of magnitude slower than glibc's
        // sprintf
        let update = format!("{:.2} {:.2} {:.2}", yaw, pitch, roll);
        self.socket.send(&update, 0).unwrap();
    }
}
