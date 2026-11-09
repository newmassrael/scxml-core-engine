// SCE-GENERATED — DO NOT EDIT
// source-hash: d4646eb3233d0326b8220cebf3753ec7bf14af5b5faee8ffa467c7536202f42b
// SCE-MAP: order_line_bounded.scxml:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package order_line_bounded

import (
	"github.com/newmassrael/sce-go-runtime/testprofile/canonical"
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

// TYPE is the name of the record Value is, as the types module of the suite's TTCN-3 export names it
// (docs/SCE_TEST_PROFILE.md §9.5): the `type` of a message of it in the test log (§12.5).
const TYPE = "order_line"

// Canonical is value in the canonical form of the test log (docs/SCE_TEST_PROFILE.md §12.5): each
// integer as the text of its digits, a byte string in upper-case hex, an enum by its declared name,
// an absent field omitted. A send of this template carries it beside the value it sends
// (`testprofile.MessageValue`).
func Canonical(value Value) any {
	return canonical.Record(canonical.Field("sku", canonical.Text(value.Sku)), canonical.Field("quantity", canonical.Integer(value.Quantity)), canonical.Field("color", canonical.Enumerated(value.Color.DeclaredName())))
}
