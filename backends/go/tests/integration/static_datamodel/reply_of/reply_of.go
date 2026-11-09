// SCE-GENERATED — DO NOT EDIT
// source-hash: 16d76c0f9ed42e7e13706b7e291287d0861ab6ac1014d873b4e04ff755d59e01
// SCE-MAP: reply_of.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: sce_runtime
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
// param_answers is the formal parameter `answers`, of type `uint32`.
//
// param_code is the formal parameter `code`, of type `uint32`.
func Matches(value Value, param_answers uint32, param_code uint32) bool {
	return value.Job == param_answers && value.Code == param_code
}

// Specific is the value the specific template `reply_of` sends (docs/SCE_TEST_PROFILE.md
// §6.2): every field it names at its value, every field it omits absent. It matches the template by
// Matches, given the same parameters. It is not named Value, as the other backends name it, since Go
// keeps the type Value and the package's functions in one namespace.
func Specific(param_answers uint32, param_code uint32) Value {
	return sce_job_reply.JobReplyPayload{Job: param_answers, Code: param_code}
}
