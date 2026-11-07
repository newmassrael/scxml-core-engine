// SCE-GENERATED — DO NOT EDIT
// source-hash: 44e42be193b6ad6075c6d0b043abdcd82b148ab78d0b09d43794619a04e8d622
// SCE-MAP: elapsed_at_most.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package elapsed_at_most

// Value is the type the template `elapsed_at_most` matches: `uint32`, as the unit of
// its type declares it.
type Value = uint32

// Matches reports whether value matches the template `elapsed_at_most` (ES 201 873-1 Annex B.1,
// docs/SCE_TEST_PROFILE.md §6.1).
//
// param_limit is the formal parameter `limit`, of type `uint32`.
func Matches(value Value, param_limit uint32) bool {
	return value <= param_limit
}
