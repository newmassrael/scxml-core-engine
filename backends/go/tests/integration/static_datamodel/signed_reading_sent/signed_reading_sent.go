// SCE-GENERATED — DO NOT EDIT
// source-hash: a75be659ab92abd1ba0e92fccd48a8e5a877548f5d15264555f23cf2a66450d0
// source: signed_reading_sent.scxml
// SCE-MAP: signed_reading_sent.scxml:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package signed_reading_sent

import (
	"github.com/newmassrael/sce-go-runtime/testprofile/canonical"
	sce_signed_reading "github.com/newmassrael/sce-go-tests/integration/static_datamodel/signed_reading"
)

// Value is the type the template `signed_reading_sent` matches: `record:SignedReading`, as the unit of
// its type declares it.
type Value = sce_signed_reading.SignedReadingPayload

// Matches reports whether value matches the template `signed_reading_sent` (ES 201 873-1 Annex B.1,
// docs/SCE_TEST_PROFILE.md §6.1).
func Matches(value Value) bool {
	return value.Delta == -7 && value.Offset == -300 && value.Level == -70000 && value.Stamp == -5000000000 && value.Total == 10000000000000000000
}

// Specific is the value the specific template `signed_reading_sent` sends (docs/SCE_TEST_PROFILE.md
// §6.2): every field it names at its value, every field it omits absent. It matches the template by
// Matches, given the same parameters. It is not named Value, as the other backends name it, since Go
// keeps the type Value and the package's functions in one namespace.
func Specific() Value {
	return sce_signed_reading.SignedReadingPayload{Delta: -7, Offset: -300, Level: -70000, Stamp: -5000000000, Total: 10000000000000000000}
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
