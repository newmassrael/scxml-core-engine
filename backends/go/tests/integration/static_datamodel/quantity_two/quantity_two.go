// SCE-GENERATED — DO NOT EDIT
// source-hash: 5dc4220e30d6a024f0c8457a0f4fbc8d751a44729f15dfcfca06523e0df0dbe4
// source: quantity_two.scxml
// SCE-MAP: quantity_two.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package quantity_two

// Value is the type the template `quantity_two` matches: `uint16`, as the unit of
// its type declares it.
type Value = uint16

// Matches reports whether value matches the template `quantity_two` (ES 201 873-1 Annex B.1,
// docs/SCE_TEST_PROFILE.md §6.1).
func Matches(value Value) bool {
	return value == 2
}

// Specific is the value the specific template `quantity_two` sends (docs/SCE_TEST_PROFILE.md
// §6.2): every field it names at its value, every field it omits absent. It matches the template by
// Matches, given the same parameters. It is not named Value, as the other backends name it, since Go
// keeps the type Value and the package's functions in one namespace.
func Specific() Value {
	return 2
}
