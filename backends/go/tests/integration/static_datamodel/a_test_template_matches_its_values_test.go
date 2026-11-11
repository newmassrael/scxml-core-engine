// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The Go units of the test profile's templates (docs/SCE_TEST_PROFILE.md §6,
// §11.4, §12.6 slice 6), matched against concrete values: each mechanism of
// ES 201 873-1 Annex B.1 the profile holds, true and false, and the value of
// each specific template matching its own template. The Go half of the Rust
// suite's `a_test_template_matches_its_values.rs` and of the Kotlin
// `ATestTemplateMatchesItsValuesTest.kt`, which hold the same templates to the
// same values, each test named `TestATestTemplateMatchesItsValues_<Case>` after
// the Rust case in Pascal case. A union (docs/adr/0092, section 7, the Go row)
// is the value struct its unit declares, built by the constructor of its case.
// The committed units beside this file are generated from the fixtures under
// `sce-build/tests/fixtures/test_profile/` and the unions they match from
// `sce-build/tests/fixtures/union/` (regen: `scripts/regen_static_datamodel_go.sh`).

package static_datamodel

import (
	"bytes"
	"reflect"
	"strings"
	"testing"

	"github.com/newmassrael/sce-forge-runtime/optional"
	sce "github.com/newmassrael/sce-go-runtime"

	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/card_payment"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/card_within"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/elapsed_at_most"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/every_matcher"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/job_report"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/job_report_of"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/job_status"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/order_customer"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/order_line"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/order_line_bounded"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/order_line_exact"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/order_payment"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/order_placed"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/order_placed_match"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/paint_color"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/report_within"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/shape_is_circle"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/shape_radius_minimal"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/shape_rect_minimal"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/union_shape_minimal"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/voucher_is"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/voucher_payment"
)

// profileEAcute is U+00E9, one character of two UTF-8 bytes.
const profileEAcute = "é"

// profileLimit is the limit every_matcher's range and reference are given.
const profileLimit uint32 = 100

// profileMatching is a job status every_matcher matches with the limit 100:
// each field at a value its matcher holds, each optional field absent.
func profileMatching() job_status.JobStatusPayload {
	return job_status.JobStatusPayload{
		ElapsedMs: 1,
		Note:      optional.None[string](),
		Codes:     []uint8{3, 9, 5},
		Tag:       optional.None[uint8](),
		Retry:     optional.None[uint8](),
		Count:     100,
	}
}

func TestATestTemplateMatchesItsValues_AStatusEachOfWhoseFieldsItsMatcherHoldsMatches(t *testing.T) {
	if !every_matcher.Matches(profileMatching(), profileLimit) {
		t.Fatal("the status every field of which its matcher holds does not match")
	}
}

// value-list (B.1.2.1) of a specific value and a range (B.1.2.5), both bounds
// inclusive, the upper one a formal parameter.
func TestATestTemplateMatchesItsValues_AValueListMatchesAnyOfItsItemsAndARangeHoldsItsBounds(t *testing.T) {
	for _, elapsed := range []uint32{1, 10, 50, profileLimit} {
		status := profileMatching()
		status.ElapsedMs = elapsed
		if !every_matcher.Matches(status, profileLimit) {
			t.Errorf("%d does not match", elapsed)
		}
	}
	for _, elapsed := range []uint32{0, 2, 9, profileLimit + 1} {
		status := profileMatching()
		status.ElapsedMs = elapsed
		if every_matcher.Matches(status, profileLimit) {
			t.Errorf("%d matches", elapsed)
		}
	}
}

// any-value with ifpresent (B.1.4.2) and a length restriction (B.1.4.1) on an
// optional text: absent, or present with at most eight characters.
func TestATestTemplateMatchesItsValues_IfpresentTakesAnAbsentFieldAndALengthCountsCharacters(t *testing.T) {
	cases := []struct {
		note optional.Of[string]
		held bool
	}{
		{optional.None[string](), true},
		{optional.Some("abcdefgh"), true},
		{optional.Some("abcdefghi"), false},
		// Eight characters of two bytes each: sixteen bytes, eight characters.
		{optional.Some(strings.Repeat(profileEAcute, 8)), true},
		{optional.Some(strings.Repeat(profileEAcute, 9)), false},
	}
	for _, c := range cases {
		status := profileMatching()
		status.Note = c.note
		if got := every_matcher.Matches(status, profileLimit); got != c.held {
			t.Errorf("note %v: matches %v, want %v", c.note, got, c.held)
		}
	}
}

// list-match (B.1.1): exactly three elements, the first 3, the second any value
// (B.1.3.1), the third not 0 (complement, B.1.2.2).
func TestATestTemplateMatchesItsValues_AListMatchHoldsItsCountAndEachElementInOrder(t *testing.T) {
	for _, items := range [][]uint8{{3, 9, 5}, {3, 0, 1}} {
		status := profileMatching()
		status.Codes = items
		if !every_matcher.Matches(status, profileLimit) {
			t.Errorf("%v does not match", items)
		}
	}
	for _, items := range [][]uint8{{3, 9, 0}, {4, 9, 5}, {3, 9}, {3, 9, 5, 5}, {}} {
		status := profileMatching()
		status.Codes = items
		if every_matcher.Matches(status, profileLimit) {
			t.Errorf("%v matches", items)
		}
	}
}

// any-value-or-none (B.1.2.4) and omit (B.1.2.8).
func TestATestTemplateMatchesItsValues_AnyValueOrNoneTakesBothAndOmitOnlyAnAbsentField(t *testing.T) {
	tagged := profileMatching()
	tagged.Tag = optional.Some[uint8](7)
	if !every_matcher.Matches(tagged, profileLimit) {
		t.Error("a present tag does not match any-value-or-none")
	}
	if !every_matcher.Matches(profileMatching(), profileLimit) {
		t.Error("an absent tag does not match any-value-or-none")
	}
	retried := profileMatching()
	retried.Retry = optional.Some[uint8](1)
	if every_matcher.Matches(retried, profileLimit) {
		t.Error("a present retry matches omit")
	}
}

// template-ref (-1 §15.1): the referenced template's match, its formal
// parameter bound to this template's own.
func TestATestTemplateMatchesItsValues_AReferenceMatchesByItsTemplateWithTheArgumentItBinds(t *testing.T) {
	status := profileMatching()
	status.Count = 150
	if every_matcher.Matches(status, profileLimit) {
		t.Error("a count past the limit matches")
	}
	if !every_matcher.Matches(status, 150) {
		t.Error("a count at the limit does not match")
	}
	if !elapsed_at_most.Matches(150, 150) {
		t.Error("150 is not at most 150")
	}
	if elapsed_at_most.Matches(151, 150) {
		t.Error("151 is at most 150")
	}
}

// A specific template's value (§6.2) matches the template, and a value that
// differs in one field does not.
func TestATestTemplateMatchesItsValues_ASpecificTemplatesValueMatchesItsTemplate(t *testing.T) {
	report := job_report.Specific()
	if report.ElapsedMs != 0 || !report.Note.Equal(optional.Some("started on time")) ||
		!reflect.DeepEqual(report.Codes, []uint8{7}) || report.Tag.IsPresent() ||
		report.Retry.IsPresent() || report.Count != 1 {
		t.Fatalf("value %+v", report)
	}
	if !job_report.Matches(report) {
		t.Fatal("the template's value does not match it")
	}

	noted := report
	noted.Note = optional.None[string]()
	if job_report.Matches(noted) {
		t.Error("an absent note matches")
	}
	retried := report
	retried.Retry = optional.Some[uint8](0)
	if job_report.Matches(retried) {
		t.Error("a present retry matches")
	}
	counted := report
	counted.Count = 2
	if job_report.Matches(counted) {
		t.Error("another count matches")
	}
}

// A specific template whose values are partly its parameters.
func TestATestTemplateMatchesItsValues_ASpecificTemplatesValueHoldsTheParametersItIsGiven(t *testing.T) {
	report := job_report_of.Specific(5, "late", 9)
	if report.ElapsedMs != 5 || !report.Note.Equal(optional.Some("late")) ||
		!reflect.DeepEqual(report.Codes, []uint8{9, 255}) || report.Tag.IsPresent() ||
		!report.Retry.Equal(optional.Some[uint8](2)) || report.Count != 4_294_967_295 {
		t.Fatalf("value %+v", report)
	}
	if !job_report_of.Matches(report, 5, "late", 9) {
		t.Fatal("the template's value does not match it with the same parameters")
	}
	if job_report_of.Matches(report, 6, "late", 9) {
		t.Error("another elapsed time matches")
	}
	if job_report_of.Matches(report, 5, "early", 9) {
		t.Error("another note matches")
	}
	if job_report_of.Matches(report, 5, "late", 8) {
		t.Error("another code matches")
	}
}

// An enum literal, and a reference to a specific template, which builds the
// field with that template's value.
func TestATestTemplateMatchesItsValues_AnEnumLiteralAndAReferenceToASpecificTemplateBuildAndMatch(t *testing.T) {
	line := order_line_exact.Specific()
	if line.Sku != "A-1" || line.Quantity != 2 || line.Color != paint_color.PaintColorRed {
		t.Fatalf("value %+v", line)
	}
	if !order_line_exact.Matches(line) {
		t.Fatal("the template's value does not match it")
	}
	blue := line
	blue.Color = paint_color.PaintColorBlue
	if order_line_exact.Matches(blue) {
		t.Error("another color matches")
	}
	three := line
	three.Quantity = 3
	if order_line_exact.Matches(three) {
		t.Error("another quantity matches")
	}
}

// A reference whose template matches a range up to the argument it binds.
func TestATestTemplateMatchesItsValues_AReferenceToARangeHoldsTheBoundItsArgumentGives(t *testing.T) {
	line := order_line.OrderLinePayload{Sku: "A-1", Quantity: 5, Color: paint_color.PaintColorBlue}
	if !order_line_bounded.Matches(line, 5) {
		t.Error("a quantity at the bound does not match")
	}
	if order_line_bounded.Matches(line, 4) {
		t.Error("a quantity past the bound matches")
	}
	green := line
	green.Color = paint_color.PaintColorGreen
	if order_line_bounded.Matches(green, 5) {
		t.Error("another color matches")
	}
}

// A template that is specific in itself and references one that is not matches
// by the referenced template's range.
func TestATestTemplateMatchesItsValues_ATemplateMatchesThroughAReferenceToOneThatIsNotSpecific(t *testing.T) {
	status := job_report.Specific()
	status.Count = 60
	if !report_within.Matches(status, 60) {
		t.Error("a count at the limit does not match")
	}
	if report_within.Matches(status, 59) {
		t.Error("a count past the limit matches")
	}
}

// A length restriction on a text counts characters, so a text whose bytes pass
// its upper bound matches while its characters do not; a range with a literal
// lower bound.
func TestATestTemplateMatchesItsValues_ALengthOnATextCountsAMultiByteCharacterAsOne(t *testing.T) {
	card := func(last4 string, amount uint32) card_payment.CardPaymentPayload {
		return card_payment.CardPaymentPayload{Last4: last4, Amount: amount}
	}
	// Two characters in three bytes, and in four.
	for _, last4 := range []string{profileEAcute + "1", strings.Repeat(profileEAcute, 2), "123"} {
		if !card_within.Matches(card(last4, 50), 100) {
			t.Errorf("%q does not match", last4)
		}
	}
	for _, last4 := range []string{"1", profileEAcute, "1234"} {
		if card_within.Matches(card(last4, 50), 100) {
			t.Errorf("%q matches", last4)
		}
	}
	amounts := []struct {
		amount uint32
		held   bool
	}{{1, true}, {0, false}, {100, true}, {101, false}}
	for _, a := range amounts {
		if got := card_within.Matches(card("12", a.amount), 100); got != a.held {
			t.Errorf("amount %d: matches %v, want %v", a.amount, got, a.held)
		}
	}
}

// A byte string written in hex, compared by its octets rather than by the
// slice that holds them.
func TestATestTemplateMatchesItsValues_AByteStringLiteralIsItsOctets(t *testing.T) {
	voucher := voucher_is.Specific()
	if !bytes.Equal(voucher.VoucherCode, []byte{0x00, 0xff, 0x10, 0xa0}) {
		t.Fatalf("code %x", voucher.VoucherCode)
	}
	if !voucher_is.Matches(voucher) {
		t.Fatal("the template's value does not match it")
	}
	// Another slice of the same octets matches too.
	if !voucher_is.Matches(voucher_payment.VoucherPaymentPayload{VoucherCode: bytes.Clone(voucher.VoucherCode)}) {
		t.Error("a copy of the octets does not match")
	}
	other := bytes.Clone(voucher.VoucherCode)
	other[3] = 0xa1
	if voucher_is.Matches(voucher_payment.VoucherPaymentPayload{VoucherCode: other}) {
		t.Error("other octets match")
	}
}

// union-match (§11.4): a union matches in the case its matcher names, and in no
// other, whatever the case holds.
func TestATestTemplateMatchesItsValues_AUnionMatchesInTheCaseItsMatcherNames(t *testing.T) {
	circle := union_shape_minimal.UnionShapeMinimalCircle(shape_radius_minimal.ShapeRadiusMinimalPayload{Radius: 3})
	if !shape_is_circle.Matches(circle) {
		t.Error("a circle does not match")
	}
	if shape_is_circle.Matches(union_shape_minimal.UnionShapeMinimalDot()) {
		t.Error("a dot matches")
	}
	rect := union_shape_minimal.UnionShapeMinimalRect(shape_rect_minimal.ShapeRectMinimalPayload{Width: 1, Height: 2})
	if shape_is_circle.Matches(rect) {
		t.Error("a rect matches")
	}
}

// profileQuantityLimit is the quantity limit order_placed_match is given.
const profileQuantityLimit uint16 = 5

// profileOrder is an order order_placed_match matches with the quantity limit 5:
// two lines, the second the exact one, no note and no priority, a VIP customer,
// paid by a card of four characters.
func profileOrder() order_placed.OrderPlacedPayload {
	return order_placed.OrderPlacedPayload{
		OrderId: 7,
		Lines: []order_line.OrderLinePayload{
			{Sku: "A-1", Quantity: 5, Color: paint_color.PaintColorGreen},
			order_line_exact.Specific(),
		},
		Note:     optional.None[string](),
		Priority: optional.None[uint8](),
		Customer: order_customer.OrderCustomerPayload{FullName: "Ada", Vip: true},
		Payment:  order_payment.OrderPaymentCard(card_payment.CardPaymentPayload{Last4: "1234", Amount: 1}),
	}
}

// A value list of two union matches: a card whose record the case's matcher
// holds, and the case that holds no record; any other case, or a card its
// matcher does not hold, matches neither.
func TestATestTemplateMatchesItsValues_AUnionFieldMatchesOneOfTheCasesItsValueListNames(t *testing.T) {
	if !order_placed_match.Matches(profileOrder(), profileQuantityLimit) {
		t.Fatal("the order paid by a card does not match")
	}
	cash := profileOrder()
	cash.Payment = order_payment.OrderPaymentCash()
	if !order_placed_match.Matches(cash, profileQuantityLimit) {
		t.Error("the order paid in cash does not match")
	}
	voucher := profileOrder()
	voucher.Payment = order_payment.OrderPaymentVoucher(voucher_is.Specific())
	if order_placed_match.Matches(voucher, profileQuantityLimit) {
		t.Error("the order paid by a voucher matches")
	}
	for _, c := range []struct {
		last4  string
		amount uint32
	}{{"123", 1}, {"1234", 0}} {
		card := profileOrder()
		card.Payment = order_payment.OrderPaymentCard(card_payment.CardPaymentPayload{Last4: c.last4, Amount: c.amount})
		if order_placed_match.Matches(card, profileQuantityLimit) {
			t.Errorf("a card of %q and %d matches", c.last4, c.amount)
		}
	}
}

// docs/SCE_TEST_PROFILE.md §12.5: a unit's Canonical writes a value by the
// types its documents declare, in the JSON Canonicalization Scheme: every
// integer as the text of its digits, a closed enum by its declared name, a byte
// string in upper-case hex, a list in order, an absent field omitted and a
// present one written, a union's case by its declared name and a case that
// holds no record as the empty object. TYPE is the unit name of the record's
// event schema. The order is the Kotlin twin's,
// anOrdersCanonicalFormIsWrittenByTheTypesItsDocumentsDeclare.
func TestATestTemplateMatchesItsValues_AValuesCanonicalFormIsWrittenByTheTypesItsDocumentsDeclare(t *testing.T) {
	jcs := func(value any) string {
		t.Helper()
		text, err := sce.WriteJCS(value)
		if err != nil {
			t.Fatalf("a canonical value is written: %v", err)
		}
		return text
	}
	tagged := profileMatching()
	tagged.Tag = optional.Some[uint8](4)
	lines := `"lines":[{"color":"green","quantity":"5","sku":"A-1"},` +
		`{"color":"red","quantity":"2","sku":"A-1"}]`
	noted := profileOrder()
	noted.Note = optional.Some("late")
	noted.Priority = optional.Some[uint8](3)
	noted.Payment = order_payment.OrderPaymentVoucher(voucher_is.Specific())
	cash := profileOrder()
	cash.Payment = order_payment.OrderPaymentCash()
	for _, c := range []struct {
		name, typeName, wantType, got, want string
	}{
		{
			"job_report", job_report.TYPE, "job_status",
			jcs(job_report.Canonical(job_report.Specific())),
			`{"codes":["7"],"count":"1","elapsed_ms":"0","note":"started on time"}`,
		},
		{
			"every_matcher", every_matcher.TYPE, "job_status",
			jcs(every_matcher.Canonical(tagged)),
			`{"codes":["3","9","5"],"count":"100","elapsed_ms":"1","tag":"4"}`,
		},
		{
			"order_line_exact", order_line_exact.TYPE, "order_line",
			jcs(order_line_exact.Canonical(order_line_exact.Specific())),
			`{"color":"red","quantity":"2","sku":"A-1"}`,
		},
		{
			"voucher_is", voucher_is.TYPE, "voucher_payment",
			jcs(voucher_is.Canonical(voucher_is.Specific())),
			`{"voucher_code":"00FF10A0"}`,
		},
		{
			"order_placed_match", order_placed_match.TYPE, "order_placed",
			jcs(order_placed_match.Canonical(profileOrder())),
			`{"customer":{"full_name":"Ada","vip":true},` + lines + `,"order_id":"7",` +
				`"payment":{"card":{"amount":"1","last4":"1234"}}}`,
		},
		{
			"order_placed_match noted", order_placed_match.TYPE, "order_placed",
			jcs(order_placed_match.Canonical(noted)),
			`{"customer":{"full_name":"Ada","vip":true},` + lines + `,"note":"late",` +
				`"order_id":"7","payment":{"voucher":{"voucher_code":"00FF10A0"}},"priority":"3"}`,
		},
		{
			"order_placed_match cash", order_placed_match.TYPE, "order_placed",
			jcs(order_placed_match.Canonical(cash)),
			`{"customer":{"full_name":"Ada","vip":true},` + lines + `,"order_id":"7",` +
				`"payment":{"cash":{}}}`,
		},
	} {
		if c.typeName != c.wantType {
			t.Errorf("%s: TYPE is %q, not %q", c.name, c.typeName, c.wantType)
		}
		if c.got != c.want {
			t.Errorf("%s: wrote %s, want %s", c.name, c.got, c.want)
		}
	}
}
