// SCE-GENERATED — DO NOT EDIT
// source-hash: 021fdf0d0c3ee151c664e2ecf6a7e5a797d68aaf3d399831e8dfd49b9e10bec9
// SCE-MAP: reply_ok.scxml:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package reply_ok

import (
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
