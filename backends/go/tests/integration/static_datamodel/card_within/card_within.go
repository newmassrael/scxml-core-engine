// SCE-GENERATED — DO NOT EDIT
// source-hash: d4646eb3233d0326b8220cebf3753ec7bf14af5b5faee8ffa467c7536202f42b
// SCE-MAP: card_within.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package card_within

import (
	"github.com/newmassrael/sce-go-runtime/testprofile"
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
