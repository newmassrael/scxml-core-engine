// SCE-GENERATED — DO NOT EDIT
// source-hash: 5610f916540cc7a66f5ffd5f3c581afae0c543595d6e705745f49fe56e124377
// source: completes_in_time.scxml
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
