// SCE-GENERATED — DO NOT EDIT
// source-hash: c4a4afbbd1df067ca57050ec11f1a2492f00b5bd54a64285853752908956ac51
// source: card_within.scxml
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

/**
 * The name of the record [Value] is, as the types module of the suite's TTCN-3 export names it
 * (docs/SCE_TEST_PROFILE.md §9.5): the `type` of a message of it in the test log (§12.5).
 */
const val TYPE: String = "card_payment"

/**
 * [value] in the canonical form of the test log (docs/SCE_TEST_PROFILE.md §12.5): each
 * integer as the text of its digits, a byte string in upper-case hex, an enum and a case
 * of a union by their declared names, an absent field omitted. A send of this template
 * carries it beside the value it sends (`com.sce.runtime.test.messageValue`).
 */
fun canonical(value: Value): Any =
    com.sce.runtime.test.Canonical.record("last4" to com.sce.runtime.test.Canonical.text(value.last4), "amount" to com.sce.runtime.test.Canonical.integer(value.amount))
