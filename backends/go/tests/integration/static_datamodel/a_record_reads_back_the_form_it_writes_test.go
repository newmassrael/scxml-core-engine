// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The reader and the writer of the canonical form a test configuration's unit
// generates for each record its system ports take (docs/SCE_TTCN3_LANE.md
// §10.3, §10.5; docs/SCE_TEST_PROFILE.md §12.5), compiled here over records of
// every form: order_placed (a list of records with a closed enum, an optional
// text and integer, a record with a truth value, a union of a case with a
// record, one with none and one with a byte string), job_status (a list of
// integers, optional integers) and form_extras (an enum of an open set, a list
// of optional elements, a 64-bit integer, a list with a lower count, an
// optional list and a list of lists). The packages are generated from
// sce-build/tests/fixtures/test_profile/trace_forms/ (regen:
// `scripts/regen_static_datamodel_go.sh`).
//
// The twin of backends/rust/tests/tests/a_record_reads_back_the_form_it_writes.rs
// and of the Kotlin ARecordReadsBackTheFormItWritesTest.kt, over the same
// configuration, with the same values and the same refusals. A value written by
// `Canonical<Record>` and read back by `Read<Record>` is the value it was,
// through the canonical text; each malformed spelling of a form is refused,
// naming where it stands; and a trace of one record of each type replays to a
// log that holds each value as the trace does.

package static_datamodel

import (
	"bytes"
	"math"
	"strings"
	"testing"

	"github.com/newmassrael/sce-forge-runtime/optional"
	sce "github.com/newmassrael/sce-go-runtime"
	"github.com/newmassrael/sce-go-runtime/testprofile"

	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/card_payment"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/form_extras"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/forms_configuration"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/job_status"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/order_customer"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/order_line"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/order_payment"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/order_placed"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/paint_color"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/payment_kind"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/replays_every_form"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/shape_kind_open"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/voucher_payment"
)

// formsLine is a line of an order.
func formsLine(sku string, quantity uint16, color paint_color.PaintColor) order_line.OrderLinePayload {
	return order_line.OrderLinePayload{Sku: sku, Quantity: quantity, Color: color}
}

// formsOrder is an order paid by payment: two lines, one with a text of a
// character of two bytes and an escaped quote, the widest order id, a note and
// no priority.
func formsOrder(payment order_payment.OrderPayment) order_placed.OrderPlacedPayload {
	return order_placed.OrderPlacedPayload{
		OrderId: math.MaxUint32,
		Lines: []order_line.OrderLinePayload{
			formsLine("A-1", 2, paint_color.PaintColorGreen),
			formsLine("\"q\" é", math.MaxUint16, paint_color.PaintColorBlue),
		},
		Note:     optional.Some("ring twice"),
		Priority: optional.None[uint8](),
		Customer: order_customer.OrderCustomerPayload{FullName: "Ada", Vip: true},
		Payment:  payment,
	}
}

// formsPayments are the three cases of the payment: one of a record, one of
// none, one of a byte string.
func formsPayments() []order_payment.OrderPayment {
	return []order_payment.OrderPayment{
		order_payment.OrderPaymentCard(card_payment.CardPaymentPayload{Last4: "4242", Amount: 1}),
		order_payment.OrderPaymentCash(),
		order_payment.OrderPaymentVoucher(voucher_payment.VoucherPaymentPayload{VoucherCode: []byte{0x00, 0xab, 0xff}}),
	}
}

func formsStatuses() []job_status.JobStatusPayload {
	return []job_status.JobStatusPayload{
		{
			ElapsedMs: 0,
			Note:      optional.None[string](),
			Codes:     []uint8{},
			Tag:       optional.Some[uint8](math.MaxUint8),
			Retry:     optional.None[uint8](),
			Count:     7,
		},
		{
			ElapsedMs: 12,
			Note:      optional.Some("late"),
			Codes:     []uint8{3, 9, 5, 0},
			Tag:       optional.None[uint8](),
			Retry:     optional.Some[uint8](0),
			Count:     0,
		},
	}
}

// formsExtras are the extras: a declared variant and a number no variant
// declares, a list with an absent element, the least and the greatest 64-bit
// integer, a list at its lower count and at its bound, an optional list
// present and absent, and a list of lists with an empty one.
func formsExtras() []form_extras.FormExtrasPayload {
	return []form_extras.FormExtrasPayload{
		{
			Shape: shape_kind_open.ShapeKindOpenCircle,
			Slots: []optional.Of[uint16]{
				optional.Some[uint16](1), optional.None[uint16](), optional.Some[uint16](math.MaxUint16),
			},
			Least: math.MinInt64,
			Flags: []bool{true},
			Spare: optional.Some([]uint8{0, 255}),
			Grid:  [][]uint8{{1, 2}, {}},
		},
		{
			Shape: shape_kind_open.ShapeKindOpenFromUnderlying(9),
			Slots: []optional.Of[uint16]{},
			Least: math.MaxInt64,
			Flags: []bool{false, true},
			Spare: optional.None[[]uint8](),
			Grid:  [][]uint8{},
		},
	}
}

// formsText is value in its canonical text, failing the test where it has
// none.
func formsText(t *testing.T, value any) string {
	t.Helper()
	text, err := sce.WriteJCS(value)
	if err != nil {
		t.Fatalf("a canonical value is written: %v", err)
	}
	return text
}

// formsParsed is text read as JSON, failing the test where it is not.
func formsParsed(t *testing.T, text string) any {
	t.Helper()
	value, err := sce.ParseJSON(text)
	if err != nil {
		t.Fatalf("%s is JSON: %v", text, err)
	}
	return value
}

// formsRoundTrip holds that value, written by write in its canonical text,
// read back from that text by read and written again, is the same text: the
// reader holds every field the writer wrote, as the writer wrote it.
func formsRoundTrip[V any](t *testing.T, value V, write func(V) any, read func(any) (V, error)) {
	t.Helper()
	written := formsText(t, write(value))
	again, err := read(formsParsed(t, written))
	if err != nil {
		t.Fatalf("%s is read back: %v", written, err)
	}
	if got := formsText(t, write(again)); got != written {
		t.Errorf("%s is read back and written as %s", written, got)
	}
}

// formsReadOrder is the order paid by payment, written in its canonical text
// and read back from it, failing the test where it is refused.
func formsReadOrder(t *testing.T, payment order_payment.OrderPayment) order_placed.OrderPlacedPayload {
	t.Helper()
	read, err := forms_configuration.ReadOrderPlaced(
		formsParsed(t, formsText(t, forms_configuration.CanonicalOrderPlaced(formsOrder(payment)))))
	if err != nil {
		t.Fatal(err)
	}
	return read
}

// docs/SCE_TEST_PROFILE.md §12.5: a value of each form reads back from the text
// its writer gives as the value it was.
func TestARecordReadsBackTheFormItWrites_AValueOfEachFormReadsBackFromItsCanonicalText(t *testing.T) {
	for _, payment := range formsPayments() {
		formsRoundTrip(t, formsOrder(payment), forms_configuration.CanonicalOrderPlaced,
			forms_configuration.ReadOrderPlaced)
	}
	for _, status := range formsStatuses() {
		formsRoundTrip(t, status, forms_configuration.CanonicalJobStatus, forms_configuration.ReadJobStatus)
	}
	for _, extra := range formsExtras() {
		formsRoundTrip(t, extra, forms_configuration.CanonicalFormExtras, forms_configuration.ReadFormExtras)
	}
	read, err := forms_configuration.ReadFormExtras(
		formsParsed(t, formsText(t, forms_configuration.CanonicalFormExtras(formsExtras()[1]))))
	if err != nil {
		t.Fatal(err)
	}
	if read.Shape.ToUnderlying() != 9 {
		t.Errorf("a number no variant declares reads as %d", read.Shape.ToUnderlying())
	}
	if read.Least != math.MaxInt64 {
		t.Errorf("the greatest int64 reads as %d", read.Least)
	}
	payments := formsPayments()
	card := formsReadOrder(t, payments[0])
	if card.Payment.Kind() != payment_kind.PaymentKindCard || card.Payment.ProvenCard().Last4 != "4242" {
		t.Errorf("the card reads as %v", card.Payment)
	}
	if cash := formsReadOrder(t, payments[1]); cash.Payment.Kind() != payment_kind.PaymentKindCash {
		t.Errorf("the cash reads as %v", cash.Payment)
	}
	voucher := formsReadOrder(t, payments[2])
	if voucher.Payment.Kind() != payment_kind.PaymentKindVoucher ||
		!bytes.Equal(voucher.Payment.ProvenVoucher().VoucherCode, []byte{0x00, 0xab, 0xff}) {
		t.Errorf("the voucher reads as %v", voucher.Payment)
	}
	if len(card.Lines) != 2 || card.Lines[1].Sku != "\"q\" é" {
		t.Errorf("the lines read as %v", card.Lines)
	}
	if card.Priority.IsPresent() {
		t.Error("an absent priority reads as absent")
	}
}

// formsRefused is the reason read refuses the canonical text of value, written
// by write, with from replaced once by to.
func formsRefused[V any](t *testing.T, value V, write func(V) any, read func(any) (V, error), from, to string) string {
	t.Helper()
	text := formsText(t, write(value))
	if !strings.Contains(text, from) {
		t.Errorf("`%s` in %s", from, text)
		return ""
	}
	changed := strings.Replace(text, from, to, 1)
	if _, err := read(formsParsed(t, changed)); err != nil {
		return err.Error()
	}
	t.Errorf("%s is read", changed)
	return ""
}

// docs/SCE_TTCN3_LANE.md §10.2: a malformed spelling of each form is refused,
// naming the place it stands at: the record's type, each field by its id and
// `[]` for an element of a list.
func TestARecordReadsBackTheFormItWrites_AMalformedSpellingOfEachFormIsRefusedByName(t *testing.T) {
	payments := formsPayments()
	card, cash, voucher := formsOrder(payments[0]), formsOrder(payments[1]), formsOrder(payments[2])
	ofOrder := func(value order_placed.OrderPlacedPayload, from, to string) string {
		return formsRefused(t, value, forms_configuration.CanonicalOrderPlaced,
			forms_configuration.ReadOrderPlaced, from, to)
	}
	extra := formsExtras()[0]
	ofExtras := func(from, to string) string {
		return formsRefused(t, extra, forms_configuration.CanonicalFormExtras,
			forms_configuration.ReadFormExtras, from, to)
	}
	status := formsStatuses()[0]
	ofStatus := func(from, to string) string {
		return formsRefused(t, status, forms_configuration.CanonicalJobStatus,
			forms_configuration.ReadJobStatus, from, to)
	}
	sku := `{"color":"red","quantity":"1","sku":"s"},`
	longName := `"full_name":"` + strings.Repeat("x", 25) + `"`
	for _, refusal := range []struct {
		reason   string
		expected string
	}{
		{ofOrder(card, `"order_id":"4294967295"`, `"order_id":"007"`),
			"order_placed.order_id: '007' is not the text of an integer's decimal digits"},
		{ofOrder(card, `"quantity":"2"`, `"quantity":"65536"`),
			"order_placed.lines[].quantity: 65536 is past the range of its type"},
		{ofOrder(card, `"vip":true`, `"vip":"true"`), "order_placed.customer.vip: a text is not a truth value"},
		{ofOrder(card, `"full_name":"Ada"`, longName),
			"order_placed.customer.full_name: a text of 25 bytes is past the 24 its type holds"},
		{ofOrder(card, `"color":"green"`, `"color":"purple"`),
			"order_placed.lines[].color: 'purple' names no variant of its enum"},
		{ofOrder(card, `"note":"ring twice"`, `"note":null`), "order_placed.note: null is not a text"},
		{ofOrder(card, `{"customer":`, `{"rush":"1","customer":`), "order_placed: 'rush' is no field of its record"},
		{ofOrder(card, `"order_id":"4294967295",`, ""),
			"order_placed.order_id: the field is absent, and its type is not optional"},
		{ofOrder(card, `"lines":[`, `"lines":[`+sku+sku+sku),
			"order_placed.lines: a list of 5 elements is outside its bounds, 0 to 4"},
		{ofOrder(card, `{"card":`, `{"cheque":`), "order_placed.payment: 'cheque' names no case of its union"},
		{ofOrder(card, `"last4":"4242"}}`, `"last4":"4242"},"cash":{}}`),
			"order_placed.payment: a value of a union is an object of one member, and this one has 2"},
		{ofOrder(cash, `{"cash":{}}`, `{"cash":{"x":"1"}}`),
			"order_placed.payment.cash: a case that holds no record is the empty object"},
		{ofOrder(voucher, `"voucher_code":"00ABFF"`, `"voucher_code":"00abff"`),
			"order_placed.payment.voucher_code: '00abff' is not octets in upper-case hex, two digits each"},
		{ofOrder(voucher, `"voucher_code":"00ABFF"`, `"voucher_code":"0011223344"`),
			"order_placed.payment.voucher_code: a byte string of 5 octets is past the 4 its type holds"},
		{ofStatus(`"tag":"255"`, `"tag":"256"`), "job_status.tag: 256 is past the range of its type"},
		{ofStatus(`"codes":[]`, `"codes":["x"]`),
			"job_status.codes[]: 'x' is not the text of an integer's decimal digits"},
		{ofExtras(`"shape":"circle"`, `"shape":"1"`),
			"form_extras.shape: 1 is the number of a declared variant, which is written by its name"},
		{ofExtras(`"shape":"circle"`, `"shape":"hexagon"`),
			"form_extras.shape: 'hexagon' names no variant of its enum, and is no number"},
		{ofExtras(`"slots":["1",null,`, `"slots":["1","x",`),
			"form_extras.slots[]: 'x' is not the text of an integer's decimal digits"},
		{ofExtras(`"flags":[true]`, `"flags":[]`),
			"form_extras.flags: a list of 0 elements is outside its bounds, 1 to 2"},
		{ofExtras(`"grid":[["1","2"]`, `"grid":[["1","2","3"]`),
			"form_extras.grid[]: a list of 3 elements is outside its bounds, 0 to 2"},
		{ofExtras(`"least":"-9223372036854775808"`, `"least":"-9223372036854775809"`),
			"form_extras.least: -9223372036854775809 is past the range of its type"},
		{ofExtras(`"spare":["0","255"]`, `"spare":null`),
			"form_extras.spare: null is not a list, which is an array"},
	} {
		if refusal.reason != refusal.expected {
			t.Errorf("refused as %q, want %q", refusal.reason, refusal.expected)
		}
	}
}

// docs/SCE_TTCN3_LANE.md §10.5: a trace of one record of each type the system
// port takes, two at the start and one later, replays to the listener, which
// takes one a round and passes; the log holds each value as the trace does.
func TestARecordReadsBackTheFormItWrites_ATraceOfEveryFormReplaysToTheValuesItHolds(t *testing.T) {
	input := func(at testprofile.Instant, messageType string, value any) testprofile.TraceInput {
		return testprofile.TraceInput{At: at, Port: "sut", MessageType: messageType, Value: value}
	}
	trace := testprofile.Trace{
		Suite:    "forms_suite",
		TestCase: replays_every_form.Name,
		Inputs: []testprofile.TraceInput{
			input(0, "order_placed", forms_configuration.CanonicalOrderPlaced(formsOrder(formsPayments()[0]))),
			input(0, "job_status", forms_configuration.CanonicalJobStatus(formsStatuses()[0])),
			input(250, "form_extras", forms_configuration.CanonicalFormExtras(formsExtras()[0])),
		},
	}
	text := trace.CanonicalText()
	suite := testprofile.Suite{
		Name:     "forms_suite",
		Executes: []testprofile.Execute{{TestCase: replays_every_form.TestCase(), Guard: testprofile.NoGuard}},
	}
	results, err := testprofile.RunSuiteWithTraces(suite, testprofile.FeaturesOf(), func(string) (testprofile.Trace, error) {
		return testprofile.ParseTrace(text)
	}, nil)
	if err != nil {
		t.Fatal(err)
	}
	if len(results) != 1 {
		t.Fatalf("the suite runs %v", suiteNames(results))
	}
	if results[0].Result.Verdict != testprofile.VerdictPass || results[0].Result.EndedAt != 250 {
		t.Fatalf("verdict %v at %d: %v", results[0].Result.Verdict, results[0].Result.EndedAt, results[0].Result.Log)
	}
	log := results[0].CanonicalLog("forms_suite")
	for _, given := range trace.Inputs {
		received := `"kind":"message-received","port":"sut","type":"` + given.MessageType + `","value":` +
			formsText(t, given.Value)
		if !strings.Contains(log, received) {
			t.Errorf("`%s` in %s", received, log)
		}
	}
}
