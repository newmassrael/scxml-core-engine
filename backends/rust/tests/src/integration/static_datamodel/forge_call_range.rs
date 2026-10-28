// SCE-GENERATED — DO NOT EDIT
// source-hash: d11fb6db69c27fed25f47a2f22a5bf7fb78da524d08f0c299e9cc553d32a6375
#![doc = "SCE-MAP: forge_call_range.scxml:7 :: _forge_body"]
// SCE-MAP: forge_call_range.scxml:7 :: _forge_body

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

pub struct ForgeCallRange {}

impl Default for ForgeCallRange {
    fn default() -> Self {
        Self::new()
    }
}

impl ForgeCallRange {
    pub fn new() -> Self {
        Self {}
    }

    #[allow(
        clippy::manual_range_contains,
        clippy::nonminimal_bool,
        clippy::neg_cmp_op_on_partial_ord,
        clippy::comparison_to_empty
    )]
    pub fn validate(&mut self, level: i32, offset: i32) -> ValidationResult {
        if level > 30 {
            return ValidationResult {
                valid: false,
                reason: "level_out_of_range".to_string(),
            };
        }
        if offset < 0 {
            return ValidationResult {
                valid: false,
                reason: "offset_out_of_range".to_string(),
            };
        }
        ValidationResult {
            valid: true,
            reason: String::new(),
        }
    }
}
