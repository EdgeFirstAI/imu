//! Crate to establish server intializations.
//! Source was found in https://github.com/erickt/rust-zmq/blob/master/examples/zguide/weather_server/main.rs
//! Publishes yaw, pitch, roll values gathered from
//! the IMU to the dedicated endpoint set.
use serde::Serialize;
use zmq::{Context, Socket};
pub struct Server {
    pub endpoint: String,
    pub socket: Socket,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub enum ImuStatus {
    Connected,
    #[serde(rename = "Not Connected")]
    #[allow(dead_code)]
    NotConnected,
}
#[derive(Serialize, Debug, Clone)]
pub struct Imu {
    pub status: ImuStatus,
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
    pub acc: f32,
    pub ax: f32,
    pub ay: f32,
    pub az: f32,
    pub gx: f32,
    pub gy: f32,
    pub gz: f32,
    pub mx: f32,
    pub my: f32,
    pub mz: f32,
    pub time: u128,
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
        rotation_update_time: u128,
    ) -> String {
        let [yaw, pitch, roll] = attitude;
        let [ax, ay, az] = accelerometer;
        let [gx, gy, gz] = gyroscope;
        let [mx, my, mz] = magnetometer;

        let data = Imu {
            status: ImuStatus::Connected,
            yaw,
            pitch,
            roll,
            acc: rot_acc,
            ax,
            ay,
            az,
            gx,
            gy,
            gz,
            mx,
            my,
            mz,
            time: rotation_update_time,
        };
        let json = serde_json::to_string(&data).unwrap();
        self.socket.send(json.as_bytes(), 0).unwrap();
        return json;
    }
}
