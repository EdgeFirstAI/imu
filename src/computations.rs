
//! Computations Module
//! Contains computations needed in relation to system
//! conversions (quaternion to euler). 
//! Contains computations needed in relation to unit 
//! conversions (radians to degrees).  

pub mod computations {
    use std::f32::consts::PI;
    const RAD_TO_DEG: f32 = 180f32 / PI;

    /// Converts quaternion parameters to euler angles. 
    pub fn quaternion2euler(
            qr: f32, qi: f32, qj: f32, qk: f32) -> (f32, f32, f32) {
        
        let sqr: f32 = qr * qr;
        let sqi: f32 = qi * qi;
        let sqj: f32 = qj * qj;
        let sqk: f32 = qk * qk;

        let yaw: f32 = (2.0 * (qi * qj + qk * qr)).atan2( sqi - sqj - sqk + sqr);
        let pitch: f32 = (-2.0 * (qi * qk - qj * qr) / (sqi + sqj + sqk + sqr)).asin();
        let roll: f32 = (2.0 * (qj * qk + qi * qr)).atan2(-sqi - sqj + sqk + sqr); 
        return (yaw, pitch, roll);
    }
    /// Converts radians to degrees for euler angles. 
    pub fn rad2degrees(angles: (f32, f32, f32)) -> (f32, f32, f32) {
        (angles.0*RAD_TO_DEG, angles.1*RAD_TO_DEG, angles.2*RAD_TO_DEG)
    }
}