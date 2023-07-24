

pub mod computations {
    use libm::{atan2f, asinf};
    use std::f32::consts::PI;

    pub fn quaternion2euler(qr:f32, qi:f32, qj:f32, qk:f32) -> (f32, f32, f32) {
        let sqr: f32 = qr*qr;
        let sqi: f32 = qi*qi;
        let sqj: f32 = qj*qj;
        let sqk: f32 = qk*qk;
    
        let yaw: f32 = atan2f(2.0 * (qi*qj + qk*qr), sqi - sqj - sqk + sqr);
        let pitch: f32 = asinf(-2.0 * (qi * qk - qj * qr) / (sqi + sqj + sqk + sqr));
        let roll: f32 = atan2f(2.0 * (qj * qk + qi * qr), -sqi - sqj + sqk + sqr);
    
        return (yaw, pitch, roll);
    }

    pub fn rad2degrees(angle: f32) -> f32 {
        angle * 180f32/PI
    }

    pub fn degrees2rad(angle: f32) -> f32 {
        angle * PI/180f32
    }

}