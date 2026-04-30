// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

use super::condition_threshold;

// pub API: validators expose `ValidationResult` to downstream consumers
// (SCE_FORGE.md §7 validator). Fixtures that don't exercise the `reason`
// field would otherwise emit dead_code.
#[derive(Debug)]
#[allow(dead_code)]
pub struct ValidationResult {
    pub valid: bool,
    pub reason: String,
}

pub struct CrossfileValidatorCondition {
    // Imported kinds (cross-file composition)
}

impl CrossfileValidatorCondition {
    pub fn new() -> Self {
        Self {
        }
    }

    pub fn validate(&mut self, coolant_temp: f64, oil_temp: f64, max_temp: f64) -> ValidationResult {
        if !(!condition_threshold::condition_threshold(coolant_temp, oil_temp, max_temp)) {
            return ValidationResult { valid: false, reason: "plausibility_failed".to_string() };
        }
        ValidationResult { valid: true, reason: String::new() }
    }
}
