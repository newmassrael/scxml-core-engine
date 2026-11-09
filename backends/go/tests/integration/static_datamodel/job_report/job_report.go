// SCE-GENERATED — DO NOT EDIT
// source-hash: 84c1b8f490600d7eecaa6c31326cf34f41eda9322dd585954c1b6ff6e016fe19
// SCE-MAP: job_report.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package job_report

import (
	"github.com/newmassrael/sce-forge-runtime/optional"
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
