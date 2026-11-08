// SCE-GENERATED — DO NOT EDIT
// source-hash: f37425ae3865d6dd180df44e52fd892d2910b6883720df85bc9f25b65e2d89d3
#![doc = "SCE-MAP: shape_is_circle.scxml:4 :: _forge_body"]
// SCE-MAP: shape_is_circle.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

/// The type the template `shape_is_circle` matches: `union:Shape`, as the
/// unit of its type declares it.
#[allow(dead_code)]
pub type Value = super::union_shape::UnionShape;

/// Whether `value` matches the template `shape_is_circle` (ES 201 873-1 Annex B.1,
/// docs/SCE_TEST_PROFILE.md §6.1).
#[allow(dead_code, unused_variables)]
pub fn matches(value: &Value) -> bool {
    matches!(value, super::union_shape::UnionShape::Circle(_))
}
