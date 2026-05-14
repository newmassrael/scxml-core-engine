#![doc = "SCE-MAP: validator_rpm_check:2"]
// SCE-MAP: validator_rpm_check:2

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

// pub API: validators expose `ValidationResult` to downstream consumers
// (SCE_FORGE.md §7 validator). Fixtures that don't exercise the `reason`
// field would otherwise emit dead_code.
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
        if rpm > 8000 {
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
