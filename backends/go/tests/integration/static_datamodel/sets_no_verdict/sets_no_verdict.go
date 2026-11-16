// SCE-GENERATED — DO NOT EDIT
// source-hash: 982a23a5ec15b8216f29e9e8e4ba6c1047dee0d66671c7835c2776a4de5a13d2
// SCE-MAP: sets_no_verdict.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-case")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package sets_no_verdict

import (
	"github.com/newmassrael/sce-go-runtime/testprofile"
	sce_unset_configuration "github.com/newmassrael/sce-go-tests/integration/static_datamodel/unset_configuration"
)

// Name is the test case's name in the suite and in the test log (docs/SCE_TEST_PROFILE.md §11.7).
const Name string = "sets_no_verdict"

// TestCase is the test case `sets_no_verdict` (docs/SCE_TEST_PROFILE.md §11.7) as the runner
// reads it: its name, its configuration and the machines of its components, the module
// parameters those components read, and the reader of the inputs a recorded trace of it carries
// (docs/SCE_TTCN3_LANE.md §10.5).
func TestCase() testprofile.TestCase {
	return testprofile.TestCase{
		Name:             Name,
		Configuration:    sce_unset_configuration.Configuration(),
		Machines:         sce_unset_configuration.Machine,
		ModuleParameters: sce_unset_configuration.ModuleParameters(),
		Inputs:           sce_unset_configuration.Input,
	}
}
