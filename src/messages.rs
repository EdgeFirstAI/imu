use std::time::Instant;
use zenoh_ros_type::{
    common_interfaces::{
        geometry_msgs::{Quaternion, Vector3},
        sensor_msgs::IMU,
        std_msgs::Header,
    },
    rcl_interfaces::builtin_interfaces::Time,
};

pub fn header(frame_id: &str, start_time: Instant) -> Header {
    Header {
        stamp: Time {
            sec: start_time.elapsed().as_secs() as i32,
            nanosec: start_time.elapsed().subsec_nanos(),
        },
        frame_id: String::from(frame_id),
    }
}

pub fn orientation(x: f64, y: f64, z: f64, w: f64) -> Quaternion {
    Quaternion { x, y, z, w }
}

pub fn angular_velocity(x: f64, y: f64, z: f64) -> Vector3 {
    Vector3 { x, y, z }
}

pub fn linear_acceleration(x: f64, y: f64, z: f64) -> Vector3 {
    Vector3 { x, y, z }
}

pub fn imu_message(
    header: Header,
    orientation: Quaternion,
    orientation_covariance: [f64; 9],
    angular_velocity: Vector3,
    angular_velocity_covariance: [f64; 9],
    linear_acceleration: Vector3,
    linear_acceleration_covariance: [f64; 9],
) -> IMU {
    IMU {
        header,
        orientation,
        orientation_covariance,
        angular_velocity,
        angular_velocity_covariance,
        linear_acceleration,
        linear_acceleration_covariance,
    }
}
