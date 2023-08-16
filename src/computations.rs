//! Computations Module
//! Contains computations needed in relation to system
//! conversions (quaternion to euler).
//! Contains computations needed in relation to unit
//! conversions (radians to degrees).  

pub mod computations {
    use std::f32::consts::PI;
    const RAD_TO_DEG: f32 = 180f32 / PI;

    /// Converts quaternion parameters to euler angles.
    /// https://stackoverflow.com/a/37560411
    pub fn quaternion2euler(qr: f32, qi: f32, qj: f32, qk: f32) -> (f32, f32, f32) {
        let yaw = (2.0 * (qk * qr + qi * qj)).atan2(-1.0 + 2.0 * (qr * qr + qi * qi));
        let pitch = (2.0 * (qj * qr - qk * qi)).asin();

        let roll = (2.0 * (qk * qj + qr * qi)).atan2(1.0 - 2.0 * (qi * qi + qj * qj));

        return (yaw, pitch, roll);
    }
    /// Converts radians to degrees for euler angles.
    pub fn rad2degrees(angles: (f32, f32, f32)) -> [f32; 3] {
        [
            angles.0 * RAD_TO_DEG,
            angles.1 * RAD_TO_DEG,
            angles.2 * RAD_TO_DEG,
        ]
    }
}
