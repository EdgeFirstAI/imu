//! Crate to establish server intializations.
//! Source was found in https://github.com/erickt/rust-zmq/blob/master/examples/zguide/weather_server/main.rs
//! Publishes yaw, pitch, roll values gathered from
//! the IMU to the dedicated endpoint set.
use zmq::{Context, Socket};

pub struct Server {
    pub endpoint: String,
    pub socket: Socket,
}

impl Server {
    /// Creates a new struct object intializing the Socket.
    pub fn new(endpoint: String) -> Self {
        let context = Context::new();
        let socket = context.socket(zmq::PUB).unwrap();
        Self { endpoint, socket }
    }

    /// Binds the socket with the dedicated endpoint set.
    pub fn start_server(&self) {
        assert!(self.socket.bind(&self.endpoint).is_ok());
    }

    /// Sends the yaw, pitch, and roll values captured by the IMU.
    pub fn send_message(
        &self,
        attitude: [f32; 3],
        accelerometer: [f32; 3],
        gyroscope: [f32; 3],
        magnetometer: [f32; 3],
        rot_acc: f32,
    ) -> String {
        // this is slower than C because the current format! implementation is
        // very, very slow. Several orders of magnitude slower than glibc's
        // sprintf
        let [yaw, pitch, roll] = attitude;
        let [ax, ay, az] = accelerometer;
        let [gx, gy, gz] = gyroscope;
        let [mx, my, mz] = magnetometer;

        let attitude_message = format!(
            "Attitude [degrees]: yaw={:.2}, pitch={:.2}, roll={:.2}, accuracy={:.2}",
            yaw, pitch, roll, rot_acc
        );

        let accelerometer_message = format!(
            "Accelerometer [m/s^2]: ax={:.2}, ay={:.2}, az={:.2}",
            ax, ay, az
        );

        let gyroscope_message = format!(
            "Gyroscope [rad/s]: gx={:.2}, gy={:.2}, gz={:.2}",
            gx, gy, gz
        );

        let magnetometer_message = format!(
            "Magnetometer [uTesla]: mx={:.2}, my={:.2}, mz={:.2}",
            mx, my, mz
        );

        let update = format!(
            "{}\n{}\n{}\n{}\n",
            attitude_message, accelerometer_message, gyroscope_message, magnetometer_message
        );
        self.socket.send(&update, 0).unwrap();
        return update;
    }
}
