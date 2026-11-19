// SCE-GENERATED — DO NOT EDIT
// source-hash: 1c3ea91c3e1e8b406e2b0bce9e01c0c8c9749b1341519f40c6354c4c7cf95fc2
// source: reply_ok.scxml
// SCE-MAP: reply_ok.scxml:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package reply_ok

import (
	"github.com/newmassrael/sce-go-runtime/testprofile/canonical"
	sce_job_reply "github.com/newmassrael/sce-go-tests/integration/static_datamodel/job_reply"
)

// Value is the type the template `reply_ok` matches: `record:JobReply`, as the unit of
// its type declares it.
type Value = sce_job_reply.JobReplyPayload

// Matches reports whether value matches the template `reply_ok` (ES 201 873-1 Annex B.1,
// docs/SCE_TEST_PROFILE.md §6.1).
func Matches(value Value) bool {
	return value.Code == 0
}

// TYPE is the name of the record Value is, as the types module of the suite's TTCN-3 export names it
// (docs/SCE_TEST_PROFILE.md §9.5): the `type` of a message of it in the test log (§12.5).
const TYPE = "job_reply"

// Canonical is value in the canonical form of the test log (docs/SCE_TEST_PROFILE.md §12.5): each
// integer as the text of its digits, a byte string in upper-case hex, an enum by its declared name,
// an absent field omitted. A send of this template carries it beside the value it sends
// (`testprofile.MessageValue`).
func Canonical(value Value) any {
	return canonical.Record(canonical.Field("job", canonical.Integer(value.Job)), canonical.Field("code", canonical.Integer(value.Code)))
}
