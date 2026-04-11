// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Do not edit — regenerate from the source SCXML file.

#[derive(Debug)]
#[allow(dead_code)]
pub struct ValidationResult {
    pub valid: bool,
    pub reason: String,
}

pub struct ValidatorRpmCheck {
    prev_rpm: u16,
}

impl ValidatorRpmCheck {
    pub fn new() -> Self {
        Self {
            prev_rpm: 0,
        }
    }

    pub fn validate(&mut self, rpm: u16, engine_state: &str) -> ValidationResult {
        if rpm < 0 || rpm > 8000 {
            return ValidationResult { valid: false, reason: "rpm_out_of_range".to_string() };
        }
        {
            let delta = if rpm > self.prev_rpm { rpm - self.prev_rpm } else { self.prev_rpm - rpm };
            if delta > 500 {
                return ValidationResult { valid: false, reason: "rpm_rate_of_change_exceeded".to_string() };
            }
        }
        if !(rpm == 0 || engine_state != "STOP") {
            return ValidationResult { valid: false, reason: "plausibility_failed".to_string() };
        }
        self.prev_rpm = rpm;
        ValidationResult { valid: true, reason: String::new() }
    }
}
