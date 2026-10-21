// SCE-GENERATED — DO NOT EDIT
// source-hash: 0fafd5dc7cb78db436777a56fff2cdefea18c3275c9e740c945e368e87cb13ce
#![doc = "SCE-MAP: forge_call_named.scxml:6 :: _forge_body"]
// SCE-MAP: forge_call_named.scxml:6 :: _forge_body

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

pub struct ForgeCallNamed {}

impl Default for ForgeCallNamed {
    fn default() -> Self {
        Self::new()
    }
}

impl ForgeCallNamed {
    pub fn new() -> Self {
        Self {}
    }

    #[allow(
        clippy::manual_range_contains,
        clippy::nonminimal_bool,
        clippy::neg_cmp_op_on_partial_ord
    )]
    pub fn validate(&mut self, name: &str) -> ValidationResult {
        if !(name != "") {
            return ValidationResult {
                valid: false,
                reason: "plausibility_failed".to_string(),
            };
        }
        ValidationResult {
            valid: true,
            reason: String::new(),
        }
    }
}
