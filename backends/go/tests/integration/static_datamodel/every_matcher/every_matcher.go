// SCE-GENERATED — DO NOT EDIT
// source-hash: 84c1b8f490600d7eecaa6c31326cf34f41eda9322dd585954c1b6ff6e016fe19
// SCE-MAP: every_matcher.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package every_matcher

import (
	"github.com/newmassrael/sce-go-runtime/testprofile"
	"github.com/newmassrael/sce-go-runtime/testprofile/canonical"
	sce_elapsed_at_most "github.com/newmassrael/sce-go-tests/integration/static_datamodel/elapsed_at_most"
	sce_job_status "github.com/newmassrael/sce-go-tests/integration/static_datamodel/job_status"
)

// Value is the type the template `every_matcher` matches: `record:JobStatus`, as the unit of
// its type declares it.
type Value = sce_job_status.JobStatusPayload

// Matches reports whether value matches the template `every_matcher` (ES 201 873-1 Annex B.1,
// docs/SCE_TEST_PROFILE.md §6.1).
//
// param_limit is the formal parameter `limit`, of type `uint32`.
func Matches(value Value, param_limit uint32) bool {
	return (value.ElapsedMs == 1 || (value.ElapsedMs >= 10 && value.ElapsedMs <= param_limit)) && (!value.Note.IsPresent() || (value.Note.IsPresent() && (!value.Note.IsPresent() || testprofile.TextLength(value.Note.Proven()) <= 8))) && len(value.Codes) == 3 && value.Codes[0] == 3 && value.Codes[2] != 0 && len(value.Codes) >= 1 && len(value.Codes) <= 4 && !value.Retry.IsPresent() && sce_elapsed_at_most.Matches(value.Count, param_limit)
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
