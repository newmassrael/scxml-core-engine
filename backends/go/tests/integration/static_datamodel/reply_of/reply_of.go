// SCE-GENERATED — DO NOT EDIT
// source-hash: 021fdf0d0c3ee151c664e2ecf6a7e5a797d68aaf3d399831e8dfd49b9e10bec9
// SCE-MAP: reply_of.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package reply_of

import (
	sce_job_reply "github.com/newmassrael/sce-go-tests/integration/static_datamodel/job_reply"
)

// Value is the type the template `reply_of` matches: `record:JobReply`, as the unit of
// its type declares it.
type Value = sce_job_reply.JobReplyPayload

// Matches reports whether value matches the template `reply_of` (ES 201 873-1 Annex B.1,
// docs/SCE_TEST_PROFILE.md §6.1).
//
// param_job is the formal parameter `job`, of type `uint32`.
//
// param_code is the formal parameter `code`, of type `uint32`.
func Matches(value Value, param_job uint32, param_code uint32) bool {
	return value.Job == param_job && value.Code == param_code
}

// Specific is the value the specific template `reply_of` sends (docs/SCE_TEST_PROFILE.md
// §6.2): every field it names at its value, every field it omits absent. It matches the template by
// Matches, given the same parameters. It is not named Value, as the other backends name it, since Go
// keeps the type Value and the package's functions in one namespace.
func Specific(param_job uint32, param_code uint32) Value {
	return sce_job_reply.JobReplyPayload{Job: param_job, Code: param_code}
}
