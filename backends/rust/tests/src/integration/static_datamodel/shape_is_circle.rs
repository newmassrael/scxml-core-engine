// SCE-GENERATED — DO NOT EDIT
// source-hash: d9a20d3d040e3f3b82b50c5dfb616ae2679f31af5d7a387c02d5d27e7c9ed9c1
#![doc = "SCE-MAP: shape_is_circle.scxml:4 :: _forge_body"]
// SCE-MAP: shape_is_circle.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

/// The type the template `shape_is_circle` matches: `union:Shape`, as the
/// unit of its type declares it.
#[allow(dead_code)]
pub type Value = super::union_shape_minimal::UnionShapeMinimal;

/// Whether `value` matches the template `shape_is_circle` (ES 201 873-1 Annex B.1,
/// docs/SCE_TEST_PROFILE.md §6.1).
#[allow(dead_code, unused_variables)]
pub fn matches(value: &Value) -> bool {
    matches!(
        value,
        super::union_shape_minimal::UnionShapeMinimal::Circle(_)
    )
}
