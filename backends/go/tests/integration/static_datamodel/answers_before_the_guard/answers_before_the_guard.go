// SCE-GENERATED — DO NOT EDIT
// source-hash: 011d804f887126b888ea6ba5b4213d56e0f5c8740555a154c83e9b8d0105c4b6
// SCE-MAP: answers_before_the_guard.scxml:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-case")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package answers_before_the_guard

import (
	"github.com/newmassrael/sce-go-runtime/testprofile"
	sce_asking_configuration "github.com/newmassrael/sce-go-tests/integration/static_datamodel/asking_configuration"
)

// Name is the test case's name in the suite and in the test log (docs/SCE_TEST_PROFILE.md §11.7).
const Name string = "answers_before_the_guard"

// TestCase is the test case `answers_before_the_guard` (docs/SCE_TEST_PROFILE.md §11.7) as the runner
// reads it: its name, its configuration and the machines of its components, and the module
// parameters those components read.
func TestCase() testprofile.TestCase {
	return testprofile.TestCase{
		Name:             Name,
		Configuration:    sce_asking_configuration.Configuration(),
		Machines:         sce_asking_configuration.Machine,
		ModuleParameters: sce_asking_configuration.ModuleParameters(),
	}
}
