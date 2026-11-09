// SCE-GENERATED — DO NOT EDIT
// source-hash: d05ce240c5f3b48b3c83b6bee1ba58feef40d57c310e9a29ef5279604bbaaa42
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
        clippy::bool_comparison,
        clippy::manual_range_contains,
        clippy::nonminimal_bool,
        clippy::neg_cmp_op_on_partial_ord,
        clippy::comparison_to_empty
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
