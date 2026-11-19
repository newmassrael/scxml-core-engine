// SCE-GENERATED — DO NOT EDIT
// source-hash: 5dc4220e30d6a024f0c8457a0f4fbc8d751a44729f15dfcfca06523e0df0dbe4
// source: quantity_two.scxml
// SCE-MAP: quantity_two.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.quantity_two

/** The type the template `quantity_two` matches: `uint16`, as the unit of its type declares it. */
typealias Value = UShort

/**
 * Whether [value] matches the template `quantity_two` (ES 201 873-1 Annex B.1,
 * docs/SCE_TEST_PROFILE.md §6.1).
 */
@Suppress("UNUSED_PARAMETER", "SENSELESS_COMPARISON", "USELESS_IS_CHECK")
fun matches(value: Value): Boolean =
    value == 2u.toUShort()

/**
 * The value the specific template `quantity_two` sends (docs/SCE_TEST_PROFILE.md
 * §6.2): every field it names at its value, every field it omits absent. It matches the
 * template by [matches], given the same parameters.
 */
@Suppress("UNUSED_PARAMETER")
fun value(): Value =
    2u.toUShort()
