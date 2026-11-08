// SCE-GENERATED — DO NOT EDIT
// source-hash: 011d804f887126b888ea6ba5b4213d56e0f5c8740555a154c83e9b8d0105c4b6
// SCE-MAP: request_of.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package request_of

import (
	sce_job_request "github.com/newmassrael/sce-go-tests/integration/static_datamodel/job_request"
)

// Value is the type the template `request_of` matches: `record:JobRequest`, as the unit of
// its type declares it.
type Value = sce_job_request.JobRequestPayload

// Matches reports whether value matches the template `request_of` (ES 201 873-1 Annex B.1,
// docs/SCE_TEST_PROFILE.md §6.1).
//
// param_job is the formal parameter `job`, of type `uint32`.
func Matches(value Value, param_job uint32) bool {
	return value.Job == param_job
}

// Specific is the value the specific template `request_of` sends (docs/SCE_TEST_PROFILE.md
// §6.2): every field it names at its value, every field it omits absent. It matches the template by
// Matches, given the same parameters. It is not named Value, as the other backends name it, since Go
// keeps the type Value and the package's functions in one namespace.
func Specific(param_job uint32) Value {
	return sce_job_request.JobRequestPayload{Job: param_job}
}
