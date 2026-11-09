// SCE-GENERATED — DO NOT EDIT
// source-hash: 84c1b8f490600d7eecaa6c31326cf34f41eda9322dd585954c1b6ff6e016fe19
// SCE-MAP: job_report_of.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package job_report_of

import (
	"github.com/newmassrael/sce-forge-runtime/optional"
	sce_job_status "github.com/newmassrael/sce-go-tests/integration/static_datamodel/job_status"
)

// Value is the type the template `job_report_of` matches: `record:JobStatus`, as the unit of
// its type declares it.
type Value = sce_job_status.JobStatusPayload

// Matches reports whether value matches the template `job_report_of` (ES 201 873-1 Annex B.1,
// docs/SCE_TEST_PROFILE.md §6.1).
//
// param_elapsed is the formal parameter `elapsed`, of type `uint32`.
//
// param_note is the formal parameter `note`, of type `string[32]`.
//
// param_code is the formal parameter `code`, of type `uint8`.
func Matches(value Value, param_elapsed uint32, param_note string, param_code uint8) bool {
	return value.ElapsedMs == param_elapsed && value.Note.IsPresent() && value.Note.Proven() == param_note && len(value.Codes) == 2 && value.Codes[0] == param_code && value.Codes[1] == 255 && !value.Tag.IsPresent() && value.Retry.IsPresent() && value.Retry.Proven() == 2 && value.Count == 4294967295
}

// Specific is the value the specific template `job_report_of` sends (docs/SCE_TEST_PROFILE.md
// §6.2): every field it names at its value, every field it omits absent. It matches the template by
// Matches, given the same parameters. It is not named Value, as the other backends name it, since Go
// keeps the type Value and the package's functions in one namespace.
func Specific(param_elapsed uint32, param_note string, param_code uint8) Value {
	return sce_job_status.JobStatusPayload{ElapsedMs: param_elapsed, Note: optional.Some[string](param_note), Codes: []uint8{param_code, 255}, Tag: optional.None[uint8](), Retry: optional.Some[uint8](2), Count: 4294967295}
}
