// SCE-GENERATED — DO NOT EDIT
// source-hash: 1e281a8396692195e709bec100279034d644b5bab0a3c6d9b461fbae9b46fce6
// source: nested_case.scxml
// SCE-MAP: nested_case.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-case")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package nested_case

import (
	"github.com/newmassrael/sce-go-runtime/testprofile"
	sce_nested_configuration "github.com/newmassrael/sce-go-tests/integration/static_datamodel/nested_configuration"
)

// Name is the test case's name in the suite and in the test log (docs/SCE_TEST_PROFILE.md §11.7).
const Name string = "nested_case"

// TestCase is the test case `nested_case` (docs/SCE_TEST_PROFILE.md §11.7) as the runner
// reads it: its name, its configuration and the machines of its components, the module
// parameters those components read, and the reader of the inputs a recorded trace of it carries
// (docs/SCE_TTCN3_LANE.md §10.5).
func TestCase() testprofile.TestCase {
	return testprofile.TestCase{
		Name:             Name,
		Configuration:    sce_nested_configuration.Configuration(),
		Machines:         sce_nested_configuration.Machine,
		ModuleParameters: sce_nested_configuration.ModuleParameters(),
		Inputs:           sce_nested_configuration.Input,
	}
}
