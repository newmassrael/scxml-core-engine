// SCE-GENERATED — DO NOT EDIT
// source-hash: 84c1b8f490600d7eecaa6c31326cf34f41eda9322dd585954c1b6ff6e016fe19
// SCE-MAP: report_within.scxml:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package report_within

import (
	sce_elapsed_at_most "github.com/newmassrael/sce-go-tests/integration/static_datamodel/elapsed_at_most"
	sce_job_status "github.com/newmassrael/sce-go-tests/integration/static_datamodel/job_status"
)

// Value is the type the template `report_within` matches: `record:JobStatus`, as the unit of
// its type declares it.
type Value = sce_job_status.JobStatusPayload

// Matches reports whether value matches the template `report_within` (ES 201 873-1 Annex B.1,
// docs/SCE_TEST_PROFILE.md §6.1).
//
// param_limit is the formal parameter `limit`, of type `uint32`.
func Matches(value Value, param_limit uint32) bool {
	return value.ElapsedMs == 0 && value.Note.IsPresent() && value.Note.Proven() == "started on time" && len(value.Codes) == 1 && value.Codes[0] == 7 && !value.Tag.IsPresent() && !value.Retry.IsPresent() && sce_elapsed_at_most.Matches(value.Count, param_limit)
}
