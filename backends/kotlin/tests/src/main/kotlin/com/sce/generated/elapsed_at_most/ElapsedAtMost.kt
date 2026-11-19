// SCE-GENERATED — DO NOT EDIT
// source-hash: 4074a1c08376b8f69f1df6ed1ed13d6d87549a8986b2065c29db9ec9711b6321
// source: elapsed_at_most.scxml
// SCE-MAP: elapsed_at_most.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.elapsed_at_most

/** The type the template `elapsed_at_most` matches: `uint32`, as the unit of its type declares it. */
typealias Value = UInt

/**
 * Whether [value] matches the template `elapsed_at_most` (ES 201 873-1 Annex B.1,
 * docs/SCE_TEST_PROFILE.md §6.1).
 *
 * @param param_limit the formal parameter `limit`, of type `uint32`.
 */
@Suppress("UNUSED_PARAMETER", "SENSELESS_COMPARISON", "USELESS_IS_CHECK")
fun matches(value: Value, param_limit: UInt): Boolean =
    value <= param_limit
