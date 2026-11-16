// SCE-GENERATED — DO NOT EDIT
// source-hash: 12a7dc72335a819ac5590a1b6b46aee0f5b664147671271baaaa11524ee4c7b9
// SCE-MAP: nested_configuration.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-configuration")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package nested_configuration

import (
	"github.com/newmassrael/sce-forge-runtime/optional"
	"github.com/newmassrael/sce-go-runtime/testprofile"
	"github.com/newmassrael/sce-go-runtime/testprofile/canonical"
	sce_nested_tester "github.com/newmassrael/sce-go-tests/integration/static_datamodel/nested_tester"
	sce_card_payment "github.com/newmassrael/sce-go-tests/integration/static_datamodel/card_payment"
	sce_order_customer "github.com/newmassrael/sce-go-tests/integration/static_datamodel/order_customer"
	sce_order_line "github.com/newmassrael/sce-go-tests/integration/static_datamodel/order_line"
	sce_order_payment "github.com/newmassrael/sce-go-tests/integration/static_datamodel/order_payment"
	sce_order_placed "github.com/newmassrael/sce-go-tests/integration/static_datamodel/order_placed"
	sce_paint_color "github.com/newmassrael/sce-go-tests/integration/static_datamodel/paint_color"
	sce_voucher_payment "github.com/newmassrael/sce-go-tests/integration/static_datamodel/voucher_payment"
)

// Configuration is the test configuration `nested_configuration` (docs/SCE_TEST_PROFILE.md
// §11.6) as the profile's runtime reads it: the MTC's component type and each PTC's, the ports
// and timers each type declares, each `<sce:connect>` and each `<sce:map>`.
func Configuration() testprofile.Configuration {
	return testprofile.Configuration{
		MTCType: "nested_tester",
		PTCTypes: map[string]string{},
		PortsOfType: map[string][]string{
			"nested_tester": {"sut"},
		},
		TimersOfType: map[string][]string{
			"nested_tester": {"tick", "limit"},
		},
		Connections: []testprofile.Connection{},
		Mappings: []testprofile.Mapping{
			{Port: testprofile.Endpoint{Component: "mtc", Port: "sut"}, SystemPort: "sut"},
		},
	}
}

// Machine is the machine of the component type componentType of the configuration: the
// component the profile's runtime drives, generated beside its test component's machine
// (docs/SCE_TEST_PROFILE.md §12.3). Nil for a type the configuration does not name, which the
// run ends with `error` where it is created. It is a testprofile.MachineFactory.
func Machine(componentType string) testprofile.ComponentMachine {
	switch componentType {
	case "nested_tester":
		return &sce_nested_tester.NestedTesterComponent{}
	}
	return nil
}

// ModuleParameters are the module parameters the components of the configuration read
// (docs/SCE_TEST_PROFILE.md §11.5), each of which a suite that runs it gives a value.
func ModuleParameters() []string {
	return []string{}
}

// ReadOrderPlaced is the record `order_placed` read from the canonical form of the test log
// (docs/SCE_TEST_PROFILE.md §12.5), as a recorded input trace holds it (docs/SCE_TTCN3_LANE.md
// §10.3), or an error naming where its value does not belong to its type. The inverse of
// CanonicalOrderPlaced.
func ReadOrderPlaced(value any) (sce_order_placed.OrderPlacedPayload, error) {
	return func() (sce_order_placed.OrderPlacedPayload, error) { object_0, err := canonical.ReadRecord(value, []string{"order_id", "lines", "note", "priority", "customer", "payment"}, "order_placed"); if err != nil { return sce_order_placed.OrderPlacedPayload{}, err }; var read sce_order_placed.OrderPlacedPayload; if read.OrderId, err = canonical.ReadInteger[uint32](canonical.Member(object_0, "order_id"), "order_placed.order_id"); err != nil { return sce_order_placed.OrderPlacedPayload{}, err }; if read.Lines, err = func() ([]sce_order_line.OrderLinePayload, error) { items_0, err := canonical.ReadElements(canonical.Member(object_0, "lines"), 0, 4, "order_placed.lines"); if err != nil { return nil, err }; read := make([]sce_order_line.OrderLinePayload, 0, len(items_0)); for _, item_0 := range items_0 { element, err := func() (sce_order_line.OrderLinePayload, error) { object_1, err := canonical.ReadRecord(item_0, []string{"sku", "quantity", "color"}, "order_placed.lines[]"); if err != nil { return sce_order_line.OrderLinePayload{}, err }; var read sce_order_line.OrderLinePayload; if read.Sku, err = canonical.ReadText(canonical.Member(object_1, "sku"), 16, "order_placed.lines[].sku"); err != nil { return sce_order_line.OrderLinePayload{}, err }; if read.Quantity, err = canonical.ReadInteger[uint16](canonical.Member(object_1, "quantity"), "order_placed.lines[].quantity"); err != nil { return sce_order_line.OrderLinePayload{}, err }; if read.Color, err = canonical.ReadEnumerated(canonical.Member(object_1, "color"), []canonical.Variant[sce_paint_color.PaintColor]{{Name: "red", Value: sce_paint_color.PaintColorRed}, {Name: "green", Value: sce_paint_color.PaintColorGreen}, {Name: "blue", Value: sce_paint_color.PaintColorBlue}}, "order_placed.lines[].color"); err != nil { return sce_order_line.OrderLinePayload{}, err }; return read, nil }(); if err != nil { return nil, err }; read = append(read, element) }; return read, nil }(); err != nil { return sce_order_placed.OrderPlacedPayload{}, err }; if read.Note, err = func(present_0 any, held bool) (optional.Of[string], error) { if !held { return optional.None[string](), nil }; read, err := canonical.ReadText(present_0, 32, "order_placed.note"); if err != nil { return optional.None[string](), err }; return optional.Some(read), nil }(object_0.Get("note")); err != nil { return sce_order_placed.OrderPlacedPayload{}, err }; if read.Priority, err = func(present_0 any, held bool) (optional.Of[uint8], error) { if !held { return optional.None[uint8](), nil }; read, err := canonical.ReadInteger[uint8](present_0, "order_placed.priority"); if err != nil { return optional.None[uint8](), err }; return optional.Some(read), nil }(object_0.Get("priority")); err != nil { return sce_order_placed.OrderPlacedPayload{}, err }; if read.Customer, err = func() (sce_order_customer.OrderCustomerPayload, error) { object_1, err := canonical.ReadRecord(canonical.Member(object_0, "customer"), []string{"full_name", "vip"}, "order_placed.customer"); if err != nil { return sce_order_customer.OrderCustomerPayload{}, err }; var read sce_order_customer.OrderCustomerPayload; if read.FullName, err = canonical.ReadText(canonical.Member(object_1, "full_name"), 24, "order_placed.customer.full_name"); err != nil { return sce_order_customer.OrderCustomerPayload{}, err }; if read.Vip, err = canonical.ReadBoolean(canonical.Member(object_1, "vip"), "order_placed.customer.vip"); err != nil { return sce_order_customer.OrderCustomerPayload{}, err }; return read, nil }(); err != nil { return sce_order_placed.OrderPlacedPayload{}, err }; if read.Payment, err = func() (sce_order_payment.OrderPayment, error) { variant_0, case_0, err := canonical.ReadUnionCase(canonical.Member(object_0, "payment"), []string{"card", "cash", "voucher"}, "order_placed.payment"); if err != nil { return sce_order_payment.OrderPayment{}, err }; switch variant_0 { case "card": read, err := func() (sce_card_payment.CardPaymentPayload, error) { object_1, err := canonical.ReadRecord(case_0, []string{"last4", "amount"}, "order_placed.payment"); if err != nil { return sce_card_payment.CardPaymentPayload{}, err }; var read sce_card_payment.CardPaymentPayload; if read.Last4, err = canonical.ReadText(canonical.Member(object_1, "last4"), 4, "order_placed.payment.last4"); err != nil { return sce_card_payment.CardPaymentPayload{}, err }; if read.Amount, err = canonical.ReadInteger[uint32](canonical.Member(object_1, "amount"), "order_placed.payment.amount"); err != nil { return sce_card_payment.CardPaymentPayload{}, err }; return read, nil }(); if err != nil { return sce_order_payment.OrderPayment{}, err }; return sce_order_payment.OrderPaymentCard(read), nil; case "cash": if err := canonical.ReadNoRecord(case_0, "order_placed.payment.cash"); err != nil { return sce_order_payment.OrderPayment{}, err }; return sce_order_payment.OrderPaymentCash(), nil; default: read, err := func() (sce_voucher_payment.VoucherPaymentPayload, error) { object_1, err := canonical.ReadRecord(case_0, []string{"voucher_code"}, "order_placed.payment"); if err != nil { return sce_voucher_payment.VoucherPaymentPayload{}, err }; var read sce_voucher_payment.VoucherPaymentPayload; if read.VoucherCode, err = canonical.ReadBytes(canonical.Member(object_1, "voucher_code"), 4, "order_placed.payment.voucher_code"); err != nil { return sce_voucher_payment.VoucherPaymentPayload{}, err }; return read, nil }(); if err != nil { return sce_order_payment.OrderPayment{}, err }; return sce_order_payment.OrderPaymentVoucher(read), nil } }(); err != nil { return sce_order_placed.OrderPlacedPayload{}, err }; return read, nil }()
}

// CanonicalOrderPlaced is the record `order_placed` in the canonical form of the test log
// (docs/SCE_TEST_PROFILE.md §12.5), which a message of it a trace hands the run carries.
func CanonicalOrderPlaced(value sce_order_placed.OrderPlacedPayload) any {
	return canonical.Record(canonical.Field("order_id", canonical.Integer(value.OrderId)), canonical.Field("lines", canonical.List(len(value.Lines), func(item_0 int) any { return canonical.Record(canonical.Field("sku", canonical.Text(value.Lines[item_0].Sku)), canonical.Field("quantity", canonical.Integer(value.Lines[item_0].Quantity)), canonical.Field("color", canonical.Enumerated(value.Lines[item_0].Color.DeclaredName()))) })), canonical.Field("note", canonical.Optional(value.Note.IsPresent(), func() any { return canonical.Text(value.Note.Proven()) })), canonical.Field("priority", canonical.Optional(value.Priority.IsPresent(), func() any { return canonical.Integer(value.Priority.Proven()) })), canonical.Field("customer", canonical.Record(canonical.Field("full_name", canonical.Text(value.Customer.FullName)), canonical.Field("vip", canonical.Boolean(value.Customer.Vip)))), canonical.Field("payment", func() any { switch value.Payment.Kind().DeclaredName().Proven() { case "card": case_0 := value.Payment.ProvenCard(); return canonical.UnionCase("card", canonical.Record(canonical.Field("last4", canonical.Text(case_0.Last4)), canonical.Field("amount", canonical.Integer(case_0.Amount)))); case "cash": return canonical.UnionCase("cash", nil); case "voucher": case_0 := value.Payment.ProvenVoucher(); return canonical.UnionCase("voucher", canonical.Record(canonical.Field("voucher_code", canonical.Bytes(case_0.VoucherCode)))); }; panic("a value of order_payment is in no case of its tag") }()))
}

// Input is the reader of the inputs of a recorded trace (docs/SCE_TTCN3_LANE.md §10.2, a
// testprofile.InputReader): the message an input of the record type typeName on the system port
// systemPort carries, its value read from the canonical form. An error, naming why, where the
// configuration has no such system port, the `in` list of its type does not name the type, or the
// value does not belong to it.
func Input(systemPort, typeName string, value any) (testprofile.Message, error) {
	if systemPort == "sut" && typeName == "order_placed" {
		read, err := ReadOrderPlaced(value)
		if err != nil {
			return testprofile.Message{}, err
		}
		return testprofile.NewMessage(sce_order_placed.EventName, read, "order_placed", CanonicalOrderPlaced), nil
	}
	return testprofile.Message{}, testprofile.NotAnInput(systemPort, typeName, []string{"sut"})
}
