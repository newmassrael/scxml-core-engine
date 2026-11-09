// SCE-GENERATED — DO NOT EDIT
// source-hash: d4646eb3233d0326b8220cebf3753ec7bf14af5b5faee8ffa467c7536202f42b
// SCE-MAP: card_within.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.card_within

/** The type the template `card_within` matches: `record:Card`, as the unit of its type declares it. */
typealias Value = com.sce.generated.card_payment.CardPaymentPayload

/**
 * Whether [value] matches the template `card_within` (ES 201 873-1 Annex B.1,
 * docs/SCE_TEST_PROFILE.md §6.1).
 *
 * @param param_most the formal parameter `most`, of type `uint32`.
 */
@Suppress("UNUSED_PARAMETER", "SENSELESS_COMPARISON", "USELESS_IS_CHECK")
fun matches(value: Value, param_most: UInt): Boolean =
    com.sce.runtime.test.textLength(value.last4) >= 2 && com.sce.runtime.test.textLength(value.last4) <= 3 && value.amount >= 1u && value.amount <= param_most
