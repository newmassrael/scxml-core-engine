// SCE-GENERATED — DO NOT EDIT
// source-hash: 8957049d6e3664213f07710e16f42947c0b405de8fa5ea61d8821d57277f69f6
// SCE-MAP: shape_is_circle.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package shape_is_circle

import (
	sce_shape_kind "github.com/newmassrael/sce-go-tests/integration/static_datamodel/shape_kind"
	sce_union_shape_minimal "github.com/newmassrael/sce-go-tests/integration/static_datamodel/union_shape_minimal"
)

// Value is the type the template `shape_is_circle` matches: `union:Shape`, as the unit of
// its type declares it.
type Value = sce_union_shape_minimal.UnionShapeMinimal

// Matches reports whether value matches the template `shape_is_circle` (ES 201 873-1 Annex B.1,
// docs/SCE_TEST_PROFILE.md §6.1).
func Matches(value Value) bool {
	return value.Kind() == sce_shape_kind.ShapeKindCircle
}
