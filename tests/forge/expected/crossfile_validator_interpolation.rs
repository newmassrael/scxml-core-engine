// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

use super::interpolation_1d_linear;

// pub API: validators expose `ValidationResult` to downstream consumers
// (SCE_FORGE.md §7 validator). Fixtures that don't exercise the `reason`
// field would otherwise emit dead_code.
#[derive(Debug)]
#[allow(dead_code)]
pub struct ValidationResult {
    pub valid: bool,
    pub reason: String,
}

pub struct CrossfileValidatorInterpolation {
    // Imported kinds (cross-file composition)
}

impl CrossfileValidatorInterpolation {
    pub fn new() -> Self {
        Self {
        }
    }

    pub fn validate(&mut self, rpm: u16) -> ValidationResult {
        if rpm < 500 || rpm > 7000 {
            return ValidationResult { valid: false, reason: "rpm_out_of_range".to_string() };
        }
        if !(interpolation_1d_linear::Interpolation1dLinear::lookup(rpm) > 200.0) {
            return ValidationResult { valid: false, reason: "plausibility_failed".to_string() };
        }
        ValidationResult { valid: true, reason: String::new() }
    }
}
