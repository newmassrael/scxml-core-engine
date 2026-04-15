// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

use super::transform_temperature;

// pub API: validators expose `ValidationResult` to downstream consumers
// (SCE_FORGE.md §7 validator). Fixtures that don't exercise the `reason`
// field would otherwise emit dead_code.
#[derive(Debug)]
#[allow(dead_code)]
pub struct ValidationResult {
    pub valid: bool,
    pub reason: String,
}

pub struct CrossfileValidatorTransform {
    // Imported kinds (cross-file composition)
}

impl CrossfileValidatorTransform {
    pub fn new() -> Self {
        Self {
        }
    }

    pub fn validate(&mut self, raw_temp: u16) -> ValidationResult {
        if raw_temp < 0 || raw_temp > 4095 {
            return ValidationResult { valid: false, reason: "raw_temp_out_of_range".to_string() };
        }
        if !(transform_temperature::compute_temperature(raw_temp) > -40.0 && transform_temperature::compute_temperature(raw_temp) < 200.0) {
            return ValidationResult { valid: false, reason: "plausibility_failed".to_string() };
        }
        ValidationResult { valid: true, reason: String::new() }
    }
}
