// SCE-GENERATED — DO NOT EDIT
// source-hash: 3c4388ecac4793a1d0d0035ff63663a7afc71b354fa02a8aa09398ea76cad0b9
// source: any_request.scxml
// SCE-MAP: any_request.scxml:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package any_request

import (
	"github.com/newmassrael/sce-go-runtime/testprofile/canonical"
	sce_job_request "github.com/newmassrael/sce-go-tests/integration/static_datamodel/job_request"
)

// Value is the type the template `any_request` matches: `record:JobRequest`, as the unit of
// its type declares it.
type Value = sce_job_request.JobRequestPayload

// Matches reports whether value matches the template `any_request` (ES 201 873-1 Annex B.1,
// docs/SCE_TEST_PROFILE.md §6.1).
func Matches(value Value) bool {
	return true
}

// TYPE is the name of the record Value is, as the types module of the suite's TTCN-3 export names it
// (docs/SCE_TEST_PROFILE.md §9.5): the `type` of a message of it in the test log (§12.5).
const TYPE = "job_request"

// Canonical is value in the canonical form of the test log (docs/SCE_TEST_PROFILE.md §12.5): each
// integer as the text of its digits, a byte string in upper-case hex, an enum by its declared name,
// an absent field omitted. A send of this template carries it beside the value it sends
// (`testprofile.MessageValue`).
func Canonical(value Value) any {
	return canonical.Record(canonical.Field("job", canonical.Integer(value.Job)))
}
