// SCE-GENERATED — DO NOT EDIT
// source-hash: d4646eb3233d0326b8220cebf3753ec7bf14af5b5faee8ffa467c7536202f42b
// SCE-MAP: order_placed_match.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.order_placed_match

/** The type the template `order_placed_match` matches: `record:OrderPlaced`, as the unit of its type declares it. */
typealias Value = com.sce.generated.order_placed.OrderPlacedPayload

/**
 * Whether [value] matches the template `order_placed_match` (ES 201 873-1 Annex B.1,
 * docs/SCE_TEST_PROFILE.md §6.1).
 *
 * @param param_limit the formal parameter `limit`, of type `uint16`.
 */
@Suppress("UNUSED_PARAMETER", "SENSELESS_COMPARISON", "USELESS_IS_CHECK")
fun matches(value: Value, param_limit: UShort): Boolean =
    value.lines.size == 2 && value.lines[0].sku == "A-1" && com.sce.generated.quantity_at_most.matches(value.lines[0].quantity, param_limit) && value.lines[0].color == com.sce.generated.paint_color.PaintColor.GREEN && com.sce.generated.order_line_exact.matches(value.lines[1]) && value.lines.size <= 4 && (value.note == null || (value.note != null && value.note.let { present_0 -> present_0 == null || com.sce.runtime.test.textLength(present_0) <= 8 })) && (value.priority == null || value.priority.let { present_0 -> present_0 != null && present_0 >= 1u.toUByte() && present_0 <= 5u.toUByte() }) && value.customer.vip && (value.payment.let { case_0 -> case_0 is com.sce.generated.order_payment.OrderPayment.Card && com.sce.runtime.test.textLength(case_0.value.last4) >= 4 && case_0.value.amount >= 1u } || value.payment is com.sce.generated.order_payment.OrderPayment.Cash)

/**
 * The name of the record [Value] is, as the types module of the suite's TTCN-3 export names it
 * (docs/SCE_TEST_PROFILE.md §9.5): the `type` of a message of it in the test log (§12.5).
 */
const val TYPE: String = "order_placed"

/**
 * [value] in the canonical form of the test log (docs/SCE_TEST_PROFILE.md §12.5): each
 * integer as the text of its digits, a byte string in upper-case hex, an enum and a case
 * of a union by their declared names, an absent field omitted. A send of this template
 * carries it beside the value it sends (`com.sce.runtime.test.messageValue`).
 */
fun canonical(value: Value): Any =
    com.sce.runtime.test.Canonical.record("order_id" to com.sce.runtime.test.Canonical.integer(value.order_id), "lines" to com.sce.runtime.test.Canonical.list(value.lines.map { item_0 -> com.sce.runtime.test.Canonical.record("sku" to com.sce.runtime.test.Canonical.text(item_0.sku), "quantity" to com.sce.runtime.test.Canonical.integer(item_0.quantity), "color" to com.sce.runtime.test.Canonical.enumerated(item_0.color.declaredName())) }), "note" to value.note?.let { present_0 -> com.sce.runtime.test.Canonical.text(present_0) }, "priority" to value.priority?.let { present_0 -> com.sce.runtime.test.Canonical.integer(present_0) }, "customer" to com.sce.runtime.test.Canonical.record("full_name" to com.sce.runtime.test.Canonical.text(value.customer.full_name), "vip" to com.sce.runtime.test.Canonical.boolean(value.customer.vip)), "payment" to when (val case_0 = value.payment) { is com.sce.generated.order_payment.OrderPayment.Card -> com.sce.runtime.test.Canonical.unionCase("card", com.sce.runtime.test.Canonical.record("last4" to com.sce.runtime.test.Canonical.text(case_0.value.last4), "amount" to com.sce.runtime.test.Canonical.integer(case_0.value.amount))); is com.sce.generated.order_payment.OrderPayment.Cash -> com.sce.runtime.test.Canonical.unionCase("cash", null); is com.sce.generated.order_payment.OrderPayment.Voucher -> com.sce.runtime.test.Canonical.unionCase("voucher", com.sce.runtime.test.Canonical.record("voucher_code" to com.sce.runtime.test.Canonical.bytes(case_0.value.voucher_code))) })
