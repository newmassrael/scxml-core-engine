// SCE-GENERATED — DO NOT EDIT
// source-hash: 84c1b8f490600d7eecaa6c31326cf34f41eda9322dd585954c1b6ff6e016fe19
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
