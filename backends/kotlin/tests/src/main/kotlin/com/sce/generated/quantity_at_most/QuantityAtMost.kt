// SCE-GENERATED — DO NOT EDIT
// source-hash: 29d573c8acf7836d816f1ff0ec9e6833df3bbd5696937f47350d29d8ef13b2f9
// source: quantity_at_most.scxml
// SCE-MAP: quantity_at_most.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.quantity_at_most

/** The type the template `quantity_at_most` matches: `uint16`, as the unit of its type declares it. */
typealias Value = UShort

/**
 * Whether [value] matches the template `quantity_at_most` (ES 201 873-1 Annex B.1,
 * docs/SCE_TEST_PROFILE.md §6.1).
 *
 * @param param_most the formal parameter `most`, of type `uint16`.
 */
@Suppress("UNUSED_PARAMETER", "SENSELESS_COMPARISON", "USELESS_IS_CHECK")
fun matches(value: Value, param_most: UShort): Boolean =
    value <= param_most
