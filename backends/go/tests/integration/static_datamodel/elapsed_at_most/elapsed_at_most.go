// SCE-GENERATED — DO NOT EDIT
// source-hash: 4074a1c08376b8f69f1df6ed1ed13d6d87549a8986b2065c29db9ec9711b6321
// source: elapsed_at_most.scxml
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
