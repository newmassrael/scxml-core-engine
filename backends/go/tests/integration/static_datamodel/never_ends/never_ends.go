// SCE-GENERATED — DO NOT EDIT
// source-hash: 86c6dd30cc120de03266ccea6cca0d437b9db87ab00fd8a8b7241bb2567cde50
// source: never_ends.scxml
// SCE-MAP: never_ends.scxml:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-case")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package never_ends

import (
	"github.com/newmassrael/sce-go-runtime/testprofile"
	sce_stalled_configuration "github.com/newmassrael/sce-go-tests/integration/static_datamodel/stalled_configuration"
)

// Name is the test case's name in the suite and in the test log (docs/SCE_TEST_PROFILE.md §11.7).
const Name string = "never_ends"

// TestCase is the test case `never_ends` (docs/SCE_TEST_PROFILE.md §11.7) as the runner
// reads it: its name, its configuration and the machines of its components, the module
// parameters those components read, and the reader of the inputs a recorded trace of it carries
// (docs/SCE_TTCN3_LANE.md §10.5).
func TestCase() testprofile.TestCase {
	return testprofile.TestCase{
		Name:             Name,
		Configuration:    sce_stalled_configuration.Configuration(),
		Machines:         sce_stalled_configuration.Machine,
		ModuleParameters: sce_stalled_configuration.ModuleParameters(),
		Inputs:           sce_stalled_configuration.Input,
	}
}
