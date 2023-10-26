use zenoh_ros_type::common_interfaces::geometry_msgs::Quaternion;
use zenoh_ros_type::common_interfaces::geometry_msgs::Vector3;
use zenoh_ros_type::rcl_interfaces::builtin_interfaces::Time;
use zenoh_ros_type::common_interfaces::std_msgs::Header;
use zenoh_ros_type::common_interfaces::sensor_msgs::IMU;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn header(frame_id: &str) -> Header {
    let time_now = SystemTime::now();
    let time_now = time_now
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
    Header {
        stamp: Time {
                sec: time_now.as_secs() as i32,
                nanosec: time_now.subsec_nanos() as u32,
            },
        frame_id: String::from(frame_id),
    }
}

pub fn orientation(x: f64, y: f64, z: f64, w: f64) -> Quaternion {
    Quaternion {
        x,
        y,
        z,
        w,
    }
}

pub fn angular_velocity(x: f64, y: f64, z: f64) -> Vector3 {
    Vector3 {
        x,
        y,
        z,
    }
}

pub fn linear_acceleration(x: f64, y: f64, z: f64) -> Vector3 {
    Vector3 {
        x,
        y,
        z,
    }
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
