// SCE-GENERATED — DO NOT EDIT
// source-hash: d4646eb3233d0326b8220cebf3753ec7bf14af5b5faee8ffa467c7536202f42b
// SCE-MAP: card_within.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package card_within

import (
	"github.com/newmassrael/sce-go-runtime/testprofile"
	"github.com/newmassrael/sce-go-runtime/testprofile/canonical"
	sce_card_payment "github.com/newmassrael/sce-go-tests/integration/static_datamodel/card_payment"
)

// Value is the type the template `card_within` matches: `record:Card`, as the unit of
// its type declares it.
type Value = sce_card_payment.CardPaymentPayload

// Matches reports whether value matches the template `card_within` (ES 201 873-1 Annex B.1,
// docs/SCE_TEST_PROFILE.md §6.1).
//
// param_most is the formal parameter `most`, of type `uint32`.
func Matches(value Value, param_most uint32) bool {
	return testprofile.TextLength(value.Last4) >= 2 && testprofile.TextLength(value.Last4) <= 3 && value.Amount >= 1 && value.Amount <= param_most
}

// TYPE is the name of the record Value is, as the types module of the suite's TTCN-3 export names it
// (docs/SCE_TEST_PROFILE.md §9.5): the `type` of a message of it in the test log (§12.5).
const TYPE = "card_payment"

// Canonical is value in the canonical form of the test log (docs/SCE_TEST_PROFILE.md §12.5): each
// integer as the text of its digits, a byte string in upper-case hex, an enum by its declared name,
// an absent field omitted. A send of this template carries it beside the value it sends
// (`testprofile.MessageValue`).
func Canonical(value Value) any {
	return canonical.Record(canonical.Field("last4", canonical.Text(value.Last4)), canonical.Field("amount", canonical.Integer(value.Amount)))
}
