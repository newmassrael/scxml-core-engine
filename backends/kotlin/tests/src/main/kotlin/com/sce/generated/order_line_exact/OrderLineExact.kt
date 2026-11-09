// SCE-GENERATED — DO NOT EDIT
// source-hash: d4646eb3233d0326b8220cebf3753ec7bf14af5b5faee8ffa467c7536202f42b
// SCE-MAP: order_line_exact.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.order_line_exact

/** The type the template `order_line_exact` matches: `record:OrderLine`, as the unit of its type declares it. */
typealias Value = com.sce.generated.order_line.OrderLinePayload

/**
 * Whether [value] matches the template `order_line_exact` (ES 201 873-1 Annex B.1,
 * docs/SCE_TEST_PROFILE.md §6.1).
 */
@Suppress("UNUSED_PARAMETER", "SENSELESS_COMPARISON", "USELESS_IS_CHECK")
fun matches(value: Value): Boolean =
    value.sku == "A-1" && com.sce.generated.quantity_two.matches(value.quantity) && value.color == com.sce.generated.paint_color.PaintColor.RED

/**
 * The value the specific template `order_line_exact` sends (docs/SCE_TEST_PROFILE.md
 * §6.2): every field it names at its value, every field it omits absent. It matches the
 * template by [matches], given the same parameters.
 */
@Suppress("UNUSED_PARAMETER")
fun value(): Value =
    com.sce.generated.order_line.OrderLinePayload(sku = "A-1", quantity = com.sce.generated.quantity_two.value(), color = com.sce.generated.paint_color.PaintColor.RED)

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
