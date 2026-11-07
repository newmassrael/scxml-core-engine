// SCE-GENERATED — DO NOT EDIT
// source-hash: d4646eb3233d0326b8220cebf3753ec7bf14af5b5faee8ffa467c7536202f42b
#![doc = "SCE-MAP: quantity_at_most.scxml:4 :: _forge_body"]
// SCE-MAP: quantity_at_most.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

/// The type the template `quantity_at_most` matches: `uint16`, as the
/// unit of its type declares it.
#[allow(dead_code)]
pub type Value = u16;

/// Whether `value` matches the template `quantity_at_most` (ES 201 873-1 Annex B.1,
/// docs/SCE_TEST_PROFILE.md §6.1).
/// - `param_most`: the formal parameter `most`, of type `uint16`.
#[allow(dead_code, unused_variables)]
pub fn matches(value: &Value, param_most: u16) -> bool {
    *value <= param_most
}
