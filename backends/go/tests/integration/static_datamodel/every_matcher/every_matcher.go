// SCE-GENERATED — DO NOT EDIT
// source-hash: 84c1b8f490600d7eecaa6c31326cf34f41eda9322dd585954c1b6ff6e016fe19
// SCE-MAP: every_matcher.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package every_matcher

import (
	"github.com/newmassrael/sce-go-runtime/testprofile"
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
