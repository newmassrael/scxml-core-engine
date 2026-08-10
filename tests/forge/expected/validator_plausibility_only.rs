#![doc = "SCE-MAP: validator_plausibility_only:2 :: _forge_body"]
// SCE-MAP: validator_plausibility_only:2 :: _forge_body

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

pub struct ValidatorPlausibilityOnly {
}

impl ValidatorPlausibilityOnly {
    pub fn new() -> Self {
        Self {
        }
    }

    pub fn validate(&mut self, voltage: f64, current: f64) -> ValidationResult {
        if !(voltage * current <= 1000.0) {
            return ValidationResult { valid: false, reason: "plausibility_failed".to_string() };
        }
        ValidationResult { valid: true, reason: String::new() }
    }
}
