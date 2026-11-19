// SCE-GENERATED — DO NOT EDIT
// source-hash: 4c07dde9a70b631ae3726315c43861e16868a63e80a69d0ff7b4427d4a0f63ae
// source: keeps_a_ptc.scxml
// SCE-MAP: keeps_a_ptc.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-case")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package keeps_a_ptc

import (
	"github.com/newmassrael/sce-go-runtime/testprofile"
	sce_keeping_configuration "github.com/newmassrael/sce-go-tests/integration/static_datamodel/keeping_configuration"
)

// Name is the test case's name in the suite and in the test log (docs/SCE_TEST_PROFILE.md §11.7).
const Name string = "keeps_a_ptc"

// TestCase is the test case `keeps_a_ptc` (docs/SCE_TEST_PROFILE.md §11.7) as the runner
// reads it: its name, its configuration and the machines of its components, the module
// parameters those components read, and the reader of the inputs a recorded trace of it carries
// (docs/SCE_TTCN3_LANE.md §10.5).
func TestCase() testprofile.TestCase {
	return testprofile.TestCase{
		Name:             Name,
		Configuration:    sce_keeping_configuration.Configuration(),
		Machines:         sce_keeping_configuration.Machine,
		ModuleParameters: sce_keeping_configuration.ModuleParameters(),
		Inputs:           sce_keeping_configuration.Input,
	}
}
