// SCE-GENERATED — DO NOT EDIT
// source-hash: 021fdf0d0c3ee151c664e2ecf6a7e5a797d68aaf3d399831e8dfd49b9e10bec9
// SCE-MAP: serves_a_ptc.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-case")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package serves_a_ptc

import (
	"github.com/newmassrael/sce-go-runtime/testprofile"
	sce_serving_configuration "github.com/newmassrael/sce-go-tests/integration/static_datamodel/serving_configuration"
)

// Name is the test case's name in the suite and in the test log (docs/SCE_TEST_PROFILE.md §11.7).
const Name string = "serves_a_ptc"

// TestCase is the test case `serves_a_ptc` (docs/SCE_TEST_PROFILE.md §11.7) as the runner
// reads it: its name, its configuration and the machines of its components, and the module
// parameters those components read.
func TestCase() testprofile.TestCase {
	return testprofile.TestCase{
		Name:             Name,
		Configuration:    sce_serving_configuration.Configuration(),
		Machines:         sce_serving_configuration.Machine,
		ModuleParameters: sce_serving_configuration.ModuleParameters(),
	}
}
