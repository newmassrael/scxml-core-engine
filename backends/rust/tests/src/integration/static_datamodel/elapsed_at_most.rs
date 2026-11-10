// SCE-GENERATED — DO NOT EDIT
// source-hash: c8f4caa94683bb9084a79d20432745e1b4d427b60033d2d08defe34d89a24446
#![doc = "SCE-MAP: elapsed_at_most.scxml:4 :: _forge_body"]
// SCE-MAP: elapsed_at_most.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

/// The type the template `elapsed_at_most` matches: `uint32`, as the
/// unit of its type declares it.
#[allow(dead_code)]
pub type Value = u32;

/// Whether `value` matches the template `elapsed_at_most` (ES 201 873-1 Annex B.1,
/// docs/SCE_TEST_PROFILE.md §6.1).
/// - `param_limit`: the formal parameter `limit`, of type `uint32`.
#[allow(dead_code, unused_variables)]
pub fn matches(value: &Value, param_limit: u32) -> bool {
    *value <= param_limit
}
