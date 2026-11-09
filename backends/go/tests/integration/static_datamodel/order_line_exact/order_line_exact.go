// SCE-GENERATED — DO NOT EDIT
// source-hash: d4646eb3233d0326b8220cebf3753ec7bf14af5b5faee8ffa467c7536202f42b
// SCE-MAP: order_line_exact.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package order_line_exact

import (
	sce_order_line "github.com/newmassrael/sce-go-tests/integration/static_datamodel/order_line"
	sce_paint_color "github.com/newmassrael/sce-go-tests/integration/static_datamodel/paint_color"
	sce_quantity_two "github.com/newmassrael/sce-go-tests/integration/static_datamodel/quantity_two"
)

// Value is the type the template `order_line_exact` matches: `record:OrderLine`, as the unit of
// its type declares it.
type Value = sce_order_line.OrderLinePayload

// Matches reports whether value matches the template `order_line_exact` (ES 201 873-1 Annex B.1,
// docs/SCE_TEST_PROFILE.md §6.1).
func Matches(value Value) bool {
	return value.Sku == "A-1" && sce_quantity_two.Matches(value.Quantity) && value.Color == sce_paint_color.PaintColorRed
}

// Specific is the value the specific template `order_line_exact` sends (docs/SCE_TEST_PROFILE.md
// §6.2): every field it names at its value, every field it omits absent. It matches the template by
// Matches, given the same parameters. It is not named Value, as the other backends name it, since Go
// keeps the type Value and the package's functions in one namespace.
func Specific() Value {
	return sce_order_line.OrderLinePayload{Sku: "A-1", Quantity: sce_quantity_two.Specific(), Color: sce_paint_color.PaintColorRed}
}
