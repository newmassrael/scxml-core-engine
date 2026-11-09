// SCE-GENERATED — DO NOT EDIT
// source-hash: 16d76c0f9ed42e7e13706b7e291287d0861ab6ac1014d873b4e04ff755d59e01
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
// reads it: its name, its configuration and the machines of its components, and the module
// parameters those components read.
func TestCase() testprofile.TestCase {
	return testprofile.TestCase{
		Name:             Name,
		Configuration:    sce_keeping_configuration.Configuration(),
		Machines:         sce_keeping_configuration.Machine,
		ModuleParameters: sce_keeping_configuration.ModuleParameters(),
	}
}
