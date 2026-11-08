// SCE-GENERATED — DO NOT EDIT
// source-hash: d9a20d3d040e3f3b82b50c5dfb616ae2679f31af5d7a387c02d5d27e7c9ed9c1
// SCE-MAP: shape_is_circle.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.shape_is_circle

/** The type the template `shape_is_circle` matches: `union:Shape`, as the unit of its type declares it. */
typealias Value = com.sce.generated.union_shape_minimal.UnionShapeMinimal

/**
 * Whether [value] matches the template `shape_is_circle` (ES 201 873-1 Annex B.1,
 * docs/SCE_TEST_PROFILE.md §6.1).
 */
@Suppress("UNUSED_PARAMETER", "SENSELESS_COMPARISON", "USELESS_IS_CHECK")
fun matches(value: Value): Boolean =
    value is com.sce.generated.union_shape_minimal.UnionShapeMinimal.Circle
