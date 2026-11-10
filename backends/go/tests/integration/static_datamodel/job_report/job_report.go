// SCE-GENERATED — DO NOT EDIT
// source-hash: 0834e4c26bd46d9ebaf430da8e162e38dd9e4bbf623718c722dcacef2754df7f
// SCE-MAP: job_report.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package job_report

import (
	"github.com/newmassrael/sce-forge-runtime/optional"
	"github.com/newmassrael/sce-go-runtime/testprofile/canonical"
	sce_job_status "github.com/newmassrael/sce-go-tests/integration/static_datamodel/job_status"
)

// Value is the type the template `job_report` matches: `record:JobStatus`, as the unit of
// its type declares it.
type Value = sce_job_status.JobStatusPayload

// Matches reports whether value matches the template `job_report` (ES 201 873-1 Annex B.1,
// docs/SCE_TEST_PROFILE.md §6.1).
func Matches(value Value) bool {
	return value.ElapsedMs == 0 && value.Note.IsPresent() && value.Note.Proven() == "started on time" && len(value.Codes) == 1 && value.Codes[0] == 7 && !value.Tag.IsPresent() && !value.Retry.IsPresent() && value.Count == 1
}

// Specific is the value the specific template `job_report` sends (docs/SCE_TEST_PROFILE.md
// §6.2): every field it names at its value, every field it omits absent. It matches the template by
// Matches, given the same parameters. It is not named Value, as the other backends name it, since Go
// keeps the type Value and the package's functions in one namespace.
func Specific() Value {
	return sce_job_status.JobStatusPayload{ElapsedMs: 0, Note: optional.Some[string]("started on time"), Codes: []uint8{7}, Tag: optional.None[uint8](), Retry: optional.None[uint8](), Count: 1}
}

// TYPE is the name of the record Value is, as the types module of the suite's TTCN-3 export names it
// (docs/SCE_TEST_PROFILE.md §9.5): the `type` of a message of it in the test log (§12.5).
const TYPE = "job_status"

// Canonical is value in the canonical form of the test log (docs/SCE_TEST_PROFILE.md §12.5): each
// integer as the text of its digits, a byte string in upper-case hex, an enum by its declared name,
// an absent field omitted. A send of this template carries it beside the value it sends
// (`testprofile.MessageValue`).
func Canonical(value Value) any {
	return canonical.Record(canonical.Field("elapsed_ms", canonical.Integer(value.ElapsedMs)), canonical.Field("note", canonical.Optional(value.Note.IsPresent(), func() any { return canonical.Text(value.Note.Proven()) })), canonical.Field("codes", canonical.List(len(value.Codes), func(item_0 int) any { return canonical.Integer(value.Codes[item_0]) })), canonical.Field("tag", canonical.Optional(value.Tag.IsPresent(), func() any { return canonical.Integer(value.Tag.Proven()) })), canonical.Field("retry", canonical.Optional(value.Retry.IsPresent(), func() any { return canonical.Integer(value.Retry.Proven()) })), canonical.Field("count", canonical.Integer(value.Count)))
}
