// SCE-GENERATED — DO NOT EDIT
// source-hash: da8cfaa8149c596ce55c8e1b8c2853ee2c20bb8f1999394376d21de2c156738a
// source: takes_before_its_exits.scxml
// SCE-MAP: takes_before_its_exits.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-case")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package takes_before_its_exits

import (
	"github.com/newmassrael/sce-go-runtime/testprofile"
	sce_leaving_configuration "github.com/newmassrael/sce-go-tests/integration/static_datamodel/leaving_configuration"
)

// Name is the test case's name in the suite and in the test log (docs/SCE_TEST_PROFILE.md §11.7).
const Name string = "takes_before_its_exits"

// TestCase is the test case `takes_before_its_exits` (docs/SCE_TEST_PROFILE.md §11.7) as the runner
// reads it: its name, its configuration and the machines of its components, the module
// parameters those components read, and the reader of the inputs a recorded trace of it carries
// (docs/SCE_TTCN3_LANE.md §10.5).
func TestCase() testprofile.TestCase {
	return testprofile.TestCase{
		Name:             Name,
		Configuration:    sce_leaving_configuration.Configuration(),
		Machines:         sce_leaving_configuration.Machine,
		ModuleParameters: sce_leaving_configuration.ModuleParameters(),
		Inputs:           sce_leaving_configuration.Input,
	}
}
