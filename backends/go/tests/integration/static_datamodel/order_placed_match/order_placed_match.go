// SCE-GENERATED — DO NOT EDIT
// source-hash: d4646eb3233d0326b8220cebf3753ec7bf14af5b5faee8ffa467c7536202f42b
// SCE-MAP: order_placed_match.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package order_placed_match

import (
	"github.com/newmassrael/sce-go-runtime/testprofile"
	"github.com/newmassrael/sce-go-runtime/testprofile/canonical"
	sce_order_line_exact "github.com/newmassrael/sce-go-tests/integration/static_datamodel/order_line_exact"
	sce_order_placed "github.com/newmassrael/sce-go-tests/integration/static_datamodel/order_placed"
	sce_paint_color "github.com/newmassrael/sce-go-tests/integration/static_datamodel/paint_color"
	sce_payment_kind "github.com/newmassrael/sce-go-tests/integration/static_datamodel/payment_kind"
	sce_quantity_at_most "github.com/newmassrael/sce-go-tests/integration/static_datamodel/quantity_at_most"
)

// Value is the type the template `order_placed_match` matches: `record:OrderPlaced`, as the unit of
// its type declares it.
type Value = sce_order_placed.OrderPlacedPayload

// Matches reports whether value matches the template `order_placed_match` (ES 201 873-1 Annex B.1,
// docs/SCE_TEST_PROFILE.md §6.1).
//
// param_limit is the formal parameter `limit`, of type `uint16`.
func Matches(value Value, param_limit uint16) bool {
	return len(value.Lines) == 2 && value.Lines[0].Sku == "A-1" && sce_quantity_at_most.Matches(value.Lines[0].Quantity, param_limit) && value.Lines[0].Color == sce_paint_color.PaintColorGreen && sce_order_line_exact.Matches(value.Lines[1]) && len(value.Lines) <= 4 && (!value.Note.IsPresent() || (value.Note.IsPresent() && (!value.Note.IsPresent() || testprofile.TextLength(value.Note.Proven()) <= 8))) && (!value.Priority.IsPresent() || (value.Priority.IsPresent() && value.Priority.Proven() >= 1 && value.Priority.Proven() <= 5)) && value.Customer.Vip && ((value.Payment.Kind() == sce_payment_kind.PaymentKindCard && testprofile.TextLength(value.Payment.ProvenCard().Last4) >= 4 && value.Payment.ProvenCard().Amount >= 1) || value.Payment.Kind() == sce_payment_kind.PaymentKindCash)
}

// TYPE is the name of the record Value is, as the types module of the suite's TTCN-3 export names it
// (docs/SCE_TEST_PROFILE.md §9.5): the `type` of a message of it in the test log (§12.5).
const TYPE = "order_placed"

// Canonical is value in the canonical form of the test log (docs/SCE_TEST_PROFILE.md §12.5): each
// integer as the text of its digits, a byte string in upper-case hex, an enum by its declared name,
// an absent field omitted. A send of this template carries it beside the value it sends
// (`testprofile.MessageValue`).
func Canonical(value Value) any {
	return canonical.Record(canonical.Field("order_id", canonical.Integer(value.OrderId)), canonical.Field("lines", canonical.List(len(value.Lines), func(item_0 int) any { return canonical.Record(canonical.Field("sku", canonical.Text(value.Lines[item_0].Sku)), canonical.Field("quantity", canonical.Integer(value.Lines[item_0].Quantity)), canonical.Field("color", canonical.Enumerated(value.Lines[item_0].Color.DeclaredName()))) })), canonical.Field("note", canonical.Optional(value.Note.IsPresent(), func() any { return canonical.Text(value.Note.Proven()) })), canonical.Field("priority", canonical.Optional(value.Priority.IsPresent(), func() any { return canonical.Integer(value.Priority.Proven()) })), canonical.Field("customer", canonical.Record(canonical.Field("full_name", canonical.Text(value.Customer.FullName)), canonical.Field("vip", canonical.Boolean(value.Customer.Vip)))), canonical.Field("payment", func() any { switch value.Payment.Kind().DeclaredName().Proven() { case "card": case_0 := value.Payment.ProvenCard(); return canonical.UnionCase("card", canonical.Record(canonical.Field("last4", canonical.Text(case_0.Last4)), canonical.Field("amount", canonical.Integer(case_0.Amount)))); case "cash": return canonical.UnionCase("cash", nil); case "voucher": case_0 := value.Payment.ProvenVoucher(); return canonical.UnionCase("voucher", canonical.Record(canonical.Field("voucher_code", canonical.Bytes(case_0.VoucherCode)))); }; panic("a value of order_payment is in no case of its tag") }()))
}
