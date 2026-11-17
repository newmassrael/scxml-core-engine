// SCE-GENERATED — DO NOT EDIT
// source-hash: 4154e2a94e5761146caffd9e99419e570b7123aab1b11fb20a04685ab39515fd
// SCE-MAP: completes_in_time.scxml:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-case")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package completes_in_time

import (
	"github.com/newmassrael/sce-go-runtime/testprofile"
	sce_tester_and_helper "github.com/newmassrael/sce-go-tests/integration/static_datamodel/tester_and_helper"
)

// Name is the test case's name in the suite and in the test log (docs/SCE_TEST_PROFILE.md §11.7).
const Name string = "completes_in_time"

// TestCase is the test case `completes_in_time` (docs/SCE_TEST_PROFILE.md §11.7) as the runner
// reads it: its name, its configuration and the machines of its components, the module
// parameters those components read, and the reader of the inputs a recorded trace of it carries
// (docs/SCE_TTCN3_LANE.md §10.5).
func TestCase() testprofile.TestCase {
	return testprofile.TestCase{
		Name:             Name,
		Configuration:    sce_tester_and_helper.Configuration(),
		Machines:         sce_tester_and_helper.Machine,
		ModuleParameters: sce_tester_and_helper.ModuleParameters(),
		Inputs:           sce_tester_and_helper.Input,
	}
}
