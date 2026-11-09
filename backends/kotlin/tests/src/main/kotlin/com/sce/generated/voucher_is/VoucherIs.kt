// SCE-GENERATED — DO NOT EDIT
// source-hash: d4646eb3233d0326b8220cebf3753ec7bf14af5b5faee8ffa467c7536202f42b
// SCE-MAP: voucher_is.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.voucher_is

/** The type the template `voucher_is` matches: `record:Voucher`, as the unit of its type declares it. */
typealias Value = com.sce.generated.voucher_payment.VoucherPaymentPayload

/**
 * Whether [value] matches the template `voucher_is` (ES 201 873-1 Annex B.1,
 * docs/SCE_TEST_PROFILE.md §6.1).
 */
@Suppress("UNUSED_PARAMETER", "SENSELESS_COMPARISON", "USELESS_IS_CHECK")
fun matches(value: Value): Boolean =
    value.voucher_code.contentEquals(byteArrayOf(0x00.toByte(), 0xff.toByte(), 0x10.toByte(), 0xa0.toByte()))

/**
 * The value the specific template `voucher_is` sends (docs/SCE_TEST_PROFILE.md
 * §6.2): every field it names at its value, every field it omits absent. It matches the
 * template by [matches], given the same parameters.
 */
@Suppress("UNUSED_PARAMETER")
fun value(): Value =
    com.sce.generated.voucher_payment.VoucherPaymentPayload(voucher_code = byteArrayOf(0x00.toByte(), 0xff.toByte(), 0x10.toByte(), 0xa0.toByte()))

/**
 * The name of the record [Value] is, as the types module of the suite's TTCN-3 export names it
 * (docs/SCE_TEST_PROFILE.md §9.5): the `type` of a message of it in the test log (§12.5).
 */
const val TYPE: String = "voucher_payment"

/**
 * [value] in the canonical form of the test log (docs/SCE_TEST_PROFILE.md §12.5): each
 * integer as the text of its digits, a byte string in upper-case hex, an enum and a case
 * of a union by their declared names, an absent field omitted. A send of this template
 * carries it beside the value it sends (`com.sce.runtime.test.messageValue`).
 */
fun canonical(value: Value): Any =
    com.sce.runtime.test.Canonical.record("voucher_code" to com.sce.runtime.test.Canonical.bytes(value.voucher_code))
