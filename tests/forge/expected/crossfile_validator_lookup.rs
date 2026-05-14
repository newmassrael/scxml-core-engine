#![doc = "SCE-MAP: crossfile_validator_lookup:7"]
// SCE-MAP: crossfile_validator_lookup:7

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

use super::lookup_severity_default;

// pub API: validators expose `ValidationResult` to downstream consumers
// (SCE_FORGE.md §7 validator). Fixtures that don't exercise the `reason`
// field would otherwise emit dead_code.
#[derive(Debug)]
#[allow(dead_code)]
pub struct ValidationResult {
    pub valid: bool,
    pub reason: String,
}

pub struct CrossfileValidatorLookup {
    // Imported kinds (cross-file composition)
}

impl CrossfileValidatorLookup {
    pub fn new() -> Self {
        Self {
        }
    }

    pub fn validate(&mut self, code: i32) -> ValidationResult {
        if code < 0 || code > 1000 {
            return ValidationResult { valid: false, reason: "code_out_of_range".to_string() };
        }
        if !(lookup_severity_default::lookup_severity(code) > 0) {
            return ValidationResult { valid: false, reason: "plausibility_failed".to_string() };
        }
        ValidationResult { valid: true, reason: String::new() }
    }
}
