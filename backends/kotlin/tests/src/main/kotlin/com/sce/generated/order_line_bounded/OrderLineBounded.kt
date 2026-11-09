// SCE-GENERATED — DO NOT EDIT
// source-hash: d4646eb3233d0326b8220cebf3753ec7bf14af5b5faee8ffa467c7536202f42b
// SCE-MAP: order_line_bounded.scxml:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.order_line_bounded

/** The type the template `order_line_bounded` matches: `record:OrderLine`, as the unit of its type declares it. */
typealias Value = com.sce.generated.order_line.OrderLinePayload

/**
 * Whether [value] matches the template `order_line_bounded` (ES 201 873-1 Annex B.1,
 * docs/SCE_TEST_PROFILE.md §6.1).
 *
 * @param param_limit the formal parameter `limit`, of type `uint16`.
 */
@Suppress("UNUSED_PARAMETER", "SENSELESS_COMPARISON", "USELESS_IS_CHECK")
fun matches(value: Value, param_limit: UShort): Boolean =
    value.sku == "A-1" && com.sce.generated.quantity_at_most.matches(value.quantity, param_limit) && value.color == com.sce.generated.paint_color.PaintColor.BLUE

/**
 * The name of the record [Value] is, as the types module of the suite's TTCN-3 export names it
 * (docs/SCE_TEST_PROFILE.md §9.5): the `type` of a message of it in the test log (§12.5).
 */
const val TYPE: String = "order_line"

/**
 * [value] in the canonical form of the test log (docs/SCE_TEST_PROFILE.md §12.5): each
 * integer as the text of its digits, a byte string in upper-case hex, an enum and a case
 * of a union by their declared names, an absent field omitted. A send of this template
 * carries it beside the value it sends (`com.sce.runtime.test.messageValue`).
 */
fun canonical(value: Value): Any =
    com.sce.runtime.test.Canonical.record("sku" to com.sce.runtime.test.Canonical.text(value.sku), "quantity" to com.sce.runtime.test.Canonical.integer(value.quantity), "color" to com.sce.runtime.test.Canonical.enumerated(value.color.declaredName()))
