// SCE-GENERATED — DO NOT EDIT
// source-hash: d4646eb3233d0326b8220cebf3753ec7bf14af5b5faee8ffa467c7536202f42b
// SCE-MAP: quantity_at_most.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package quantity_at_most

// Value is the type the template `quantity_at_most` matches: `uint16`, as the unit of
// its type declares it.
type Value = uint16

// Matches reports whether value matches the template `quantity_at_most` (ES 201 873-1 Annex B.1,
// docs/SCE_TEST_PROFILE.md §6.1).
//
// param_most is the formal parameter `most`, of type `uint16`.
func Matches(value Value, param_most uint16) bool {
	return value <= param_most
}
