// SCE-GENERATED — DO NOT EDIT
// source-hash: d4646eb3233d0326b8220cebf3753ec7bf14af5b5faee8ffa467c7536202f42b
// SCE-MAP: order_line_bounded.scxml:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package order_line_bounded

import (
	sce_order_line "github.com/newmassrael/sce-go-tests/integration/static_datamodel/order_line"
	sce_paint_color "github.com/newmassrael/sce-go-tests/integration/static_datamodel/paint_color"
	sce_quantity_at_most "github.com/newmassrael/sce-go-tests/integration/static_datamodel/quantity_at_most"
)

// Value is the type the template `order_line_bounded` matches: `record:OrderLine`, as the unit of
// its type declares it.
type Value = sce_order_line.OrderLinePayload

// Matches reports whether value matches the template `order_line_bounded` (ES 201 873-1 Annex B.1,
// docs/SCE_TEST_PROFILE.md §6.1).
//
// param_limit is the formal parameter `limit`, of type `uint16`.
func Matches(value Value, param_limit uint16) bool {
	return value.Sku == "A-1" && sce_quantity_at_most.Matches(value.Quantity, param_limit) && value.Color == sce_paint_color.PaintColorBlue
}
