// SCE-GENERATED — DO NOT EDIT
// source-hash: d4646eb3233d0326b8220cebf3753ec7bf14af5b5faee8ffa467c7536202f42b
// SCE-MAP: order_placed_match.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: none
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
