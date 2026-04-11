// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Do not edit — regenerate from the source SCXML file.

#[derive(Debug)]
#[allow(dead_code)]
pub struct ValidationResult {
    pub valid: bool,
    pub reason: String,
}

pub struct ValidatorSignedRoc {
    prev_speed: i32,
    prev_altitude: f64,
}

impl ValidatorSignedRoc {
    pub fn new() -> Self {
        Self {
            prev_speed: 0,
            prev_altitude: 0.0,
        }
    }

    pub fn validate(&mut self, speed: i32, altitude: f64) -> ValidationResult {
        if speed < -100 || speed > 500 {
            return ValidationResult { valid: false, reason: "speed_out_of_range".to_string() };
        }
        if altitude > 50000.0 {
            return ValidationResult { valid: false, reason: "altitude_out_of_range".to_string() };
        }
        {
            let delta = (speed - self.prev_speed).unsigned_abs();
            if delta > 50 {
                return ValidationResult { valid: false, reason: "speed_rate_of_change_exceeded".to_string() };
            }
        }
        {
            let delta = (altitude - self.prev_altitude).abs();
            if delta > 100.0 {
                return ValidationResult { valid: false, reason: "altitude_rate_of_change_exceeded".to_string() };
            }
        }
        self.prev_speed = speed;
        self.prev_altitude = altitude;
        ValidationResult { valid: true, reason: String::new() }
    }
}
