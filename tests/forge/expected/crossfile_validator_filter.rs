// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

use super::filter_low_pass::FilterLowPass;

// pub API: validators expose `ValidationResult` to downstream consumers
// (SCE_FORGE.md §7 validator). Fixtures that don't exercise the `reason`
// field would otherwise emit dead_code.
#[derive(Debug)]
#[allow(dead_code)]
pub struct ValidationResult {
    pub valid: bool,
    pub reason: String,
}

pub struct CrossfileValidatorFilter {
    // Imported kinds (cross-file composition)
    pub smoother: FilterLowPass,
}

impl CrossfileValidatorFilter {
    pub fn new() -> Self {
        Self {
            smoother: FilterLowPass::new(),
        }
    }

    pub fn validate(&mut self, raw_sample: f64, threshold: f64) -> ValidationResult {
        if !(self.smoother.update(raw_sample) < threshold) {
            return ValidationResult { valid: false, reason: "plausibility_failed".to_string() };
        }
        ValidationResult { valid: true, reason: String::new() }
    }
}
