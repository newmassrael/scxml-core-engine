// SCE-GENERATED — DO NOT EDIT
// source-hash: d4646eb3233d0326b8220cebf3753ec7bf14af5b5faee8ffa467c7536202f42b
// SCE-MAP: order_payment.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="union")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package order_payment

import (
	sce_payment_kind "github.com/newmassrael/sce-go-tests/integration/static_datamodel/payment_kind"
	sce_card_payment "github.com/newmassrael/sce-go-tests/integration/static_datamodel/card_payment"
	sce_voucher_payment "github.com/newmassrael/sce-go-tests/integration/static_datamodel/voucher_payment"
)

// Revision is the revision of the union: its tag enum's, `payment_kind`'s (a union declares none of its own).
const Revision uint32 = 1

// DataMaxLen is the most bytes a value's JSON text takes: its largest case, every list full,
// every text at its bound, every number at its longest spelling.
const DataMaxLen uint64 = 70

// SchemaID is the id of the union's schema, `urn:sce:schema:<document>:<revision>`: the
// `$id` of JSONSchema.
const SchemaID string = "urn:sce:schema:order_payment:1"

// JSONSchema is a value as a JSON Schema (2020-12): the `oneOf` of the cases, each its tag
// `kind` beside the fields of its record, every bound as this build resolved it.
const JSONSchema string = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:order_payment:1\",\"x-sce-source-hash\":\"d4646eb3233d0326b8220cebf3753ec7bf14af5b5faee8ffa467c7536202f42b\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"oneOf\":[{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"card\"},\"last4\":{\"type\":\"string\",\"maxLength\":4,\"x-sce-max-bytes\":4},\"amount\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295}},\"required\":[\"kind\",\"last4\",\"amount\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"cash\"}},\"required\":[\"kind\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"voucher\"},\"voucher_code\":{\"type\":\"string\",\"maxLength\":4,\"pattern\":\"^[\\\\u0000-\\\\u00ff]*$\"}},\"required\":[\"kind\",\"voucher_code\"],\"additionalProperties\":false}]}"

// OrderPayment is a value of `order_payment`: exactly one case, told apart by its tag
// `kind`, a value of sce_payment_kind.PaymentKind. It is built only by the constructor of its case and
// holds a member for the record of each case, so it is copied by value and shares nothing
// with the record it was built from.
type OrderPayment struct {
	tag sce_payment_kind.PaymentKind
	caseCard sce_card_payment.CardPaymentPayload
	caseVoucher sce_voucher_payment.VoucherPaymentPayload
}

// Kind is the case this value is in: the tag its text writes under `kind`.
func (u OrderPayment) Kind() sce_payment_kind.PaymentKind {
	return u.tag
}

// OrderPaymentCard is the case `card`, holding its record.
func OrderPaymentCard(r sce_card_payment.CardPaymentPayload) OrderPayment {
	return OrderPayment{tag: sce_payment_kind.PaymentKindCard, caseCard: r}
}

// OrderPaymentCash is the case `cash`, its tag alone.
func OrderPaymentCash() OrderPayment {
	return OrderPayment{tag: sce_payment_kind.PaymentKindCash}
}

// OrderPaymentVoucher is the case `voucher`, holding its record.
func OrderPaymentVoucher(r sce_voucher_payment.VoucherPaymentPayload) OrderPayment {
	return OrderPayment{tag: sce_payment_kind.PaymentKindVoucher, caseVoucher: r}
}

// ProvenCard is the record of the case `card`, read where a test of the tag
// proved the value is in it.
func (u OrderPayment) ProvenCard() sce_card_payment.CardPaymentPayload {
	if u.tag != sce_payment_kind.PaymentKindCard {
		panic("a value of order_payment read as card is in another case")
	}
	return u.caseCard
}

// ProvenVoucher is the record of the case `voucher`, read where a test of the tag
// proved the value is in it.
func (u OrderPayment) ProvenVoucher() sce_voucher_payment.VoucherPaymentPayload {
	if u.tag != sce_payment_kind.PaymentKindVoucher {
		panic("a value of order_payment read as voucher is in another case")
	}
	return u.caseVoucher
}
