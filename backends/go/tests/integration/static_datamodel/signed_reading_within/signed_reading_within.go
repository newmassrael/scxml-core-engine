// SCE-GENERATED — DO NOT EDIT
// source-hash: 982a23a5ec15b8216f29e9e8e4ba6c1047dee0d66671c7835c2776a4de5a13d2
// SCE-MAP: signed_reading_within.scxml:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package signed_reading_within

import (
	"github.com/newmassrael/sce-go-runtime/testprofile/canonical"
	sce_signed_reading "github.com/newmassrael/sce-go-tests/integration/static_datamodel/signed_reading"
)

// Value is the type the template `signed_reading_within` matches: `record:SignedReading`, as the unit of
// its type declares it.
type Value = sce_signed_reading.SignedReadingPayload

// Matches reports whether value matches the template `signed_reading_within` (ES 201 873-1 Annex B.1,
// docs/SCE_TEST_PROFILE.md §6.1).
func Matches(value Value) bool {
	return value.Delta <= -1 && value.Level <= -1 && value.Stamp <= -4294967296 && value.Total >= 9223372036854775808
}

// TYPE is the name of the record Value is, as the types module of the suite's TTCN-3 export names it
// (docs/SCE_TEST_PROFILE.md §9.5): the `type` of a message of it in the test log (§12.5).
const TYPE = "signed_reading"

// Canonical is value in the canonical form of the test log (docs/SCE_TEST_PROFILE.md §12.5): each
// integer as the text of its digits, a byte string in upper-case hex, an enum by its declared name,
// an absent field omitted. A send of this template carries it beside the value it sends
// (`testprofile.MessageValue`).
func Canonical(value Value) any {
	return canonical.Record(canonical.Field("delta", canonical.Integer(value.Delta)), canonical.Field("offset", canonical.Integer(value.Offset)), canonical.Field("level", canonical.Integer(value.Level)), canonical.Field("stamp", canonical.Integer(value.Stamp)), canonical.Field("total", canonical.Integer(value.Total)))
}
