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

impl Default for ValidatorPlausibilityOnly {
    fn default() -> Self {
        Self::new()
    }
}

impl ValidatorPlausibilityOnly {
    pub fn new() -> Self {
        Self {
        }
    }

    #[allow(
        clippy::bool_comparison,
        clippy::manual_range_contains,
        clippy::nonminimal_bool,
        clippy::neg_cmp_op_on_partial_ord,
        clippy::comparison_to_empty
    )]
    pub fn validate(&mut self, voltage: f64, current: f64) -> ValidationResult {
        if !(voltage * current <= 1000.0) {
            return ValidationResult { valid: false, reason: "plausibility_failed".to_string() };
        }
        ValidationResult { valid: true, reason: String::new() }
    }
}
