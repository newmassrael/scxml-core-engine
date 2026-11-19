// SCE-GENERATED — DO NOT EDIT
// source-hash: 7abea7973bcdc2d068064b2dd6062cfe96348ca03b00db9f639c12ef4c2a89b3
// source: answers_another_job.scxml
// SCE-MAP: answers_another_job.scxml:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-case")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package answers_another_job

import (
	"github.com/newmassrael/sce-go-runtime/testprofile"
	sce_asking_configuration "github.com/newmassrael/sce-go-tests/integration/static_datamodel/asking_configuration"
)

// Name is the test case's name in the suite and in the test log (docs/SCE_TEST_PROFILE.md §11.7).
const Name string = "answers_another_job"

// TestCase is the test case `answers_another_job` (docs/SCE_TEST_PROFILE.md §11.7) as the runner
// reads it: its name, its configuration and the machines of its components, the module
// parameters those components read, and the reader of the inputs a recorded trace of it carries
// (docs/SCE_TTCN3_LANE.md §10.5).
func TestCase() testprofile.TestCase {
	return testprofile.TestCase{
		Name:             Name,
		Configuration:    sce_asking_configuration.Configuration(),
		Machines:         sce_asking_configuration.Machine,
		ModuleParameters: sce_asking_configuration.ModuleParameters(),
		Inputs:           sce_asking_configuration.Input,
	}
}
