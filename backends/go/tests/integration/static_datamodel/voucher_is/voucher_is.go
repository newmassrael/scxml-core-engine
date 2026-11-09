// SCE-GENERATED — DO NOT EDIT
// source-hash: d4646eb3233d0326b8220cebf3753ec7bf14af5b5faee8ffa467c7536202f42b
// SCE-MAP: voucher_is.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package voucher_is

import (
	"bytes"
	"github.com/newmassrael/sce-go-runtime/testprofile/canonical"
	sce_voucher_payment "github.com/newmassrael/sce-go-tests/integration/static_datamodel/voucher_payment"
)

// Value is the type the template `voucher_is` matches: `record:Voucher`, as the unit of
// its type declares it.
type Value = sce_voucher_payment.VoucherPaymentPayload

// Matches reports whether value matches the template `voucher_is` (ES 201 873-1 Annex B.1,
// docs/SCE_TEST_PROFILE.md §6.1).
func Matches(value Value) bool {
	return bytes.Equal(value.VoucherCode, []byte{0x00, 0xff, 0x10, 0xa0})
}

// Specific is the value the specific template `voucher_is` sends (docs/SCE_TEST_PROFILE.md
// §6.2): every field it names at its value, every field it omits absent. It matches the template by
// Matches, given the same parameters. It is not named Value, as the other backends name it, since Go
// keeps the type Value and the package's functions in one namespace.
func Specific() Value {
	return sce_voucher_payment.VoucherPaymentPayload{VoucherCode: []byte{0x00, 0xff, 0x10, 0xa0}}
}

// TYPE is the name of the record Value is, as the types module of the suite's TTCN-3 export names it
// (docs/SCE_TEST_PROFILE.md §9.5): the `type` of a message of it in the test log (§12.5).
const TYPE = "voucher_payment"

// Canonical is value in the canonical form of the test log (docs/SCE_TEST_PROFILE.md §12.5): each
// integer as the text of its digits, a byte string in upper-case hex, an enum by its declared name,
// an absent field omitted. A send of this template carries it beside the value it sends
// (`testprofile.MessageValue`).
func Canonical(value Value) any {
	return canonical.Record(canonical.Field("voucher_code", canonical.Bytes(value.VoucherCode)))
}
