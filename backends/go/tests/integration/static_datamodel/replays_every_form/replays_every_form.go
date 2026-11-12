// SCE-GENERATED — DO NOT EDIT
// source-hash: 72891728be38ed1db13e4394197bad5a7eaecc0eeac062f21e8fdc061c4ae890
// SCE-MAP: replays_every_form.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-case")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package replays_every_form

import (
	"github.com/newmassrael/sce-go-runtime/testprofile"
	sce_forms_configuration "github.com/newmassrael/sce-go-tests/integration/static_datamodel/forms_configuration"
)

// Name is the test case's name in the suite and in the test log (docs/SCE_TEST_PROFILE.md §11.7).
const Name string = "replays_every_form"

// TestCase is the test case `replays_every_form` (docs/SCE_TEST_PROFILE.md §11.7) as the runner
// reads it: its name, its configuration and the machines of its components, the module
// parameters those components read, and the reader of the inputs a recorded trace of it carries
// (docs/SCE_TTCN3_LANE.md §10.5).
func TestCase() testprofile.TestCase {
	return testprofile.TestCase{
		Name:             Name,
		Configuration:    sce_forms_configuration.Configuration(),
		Machines:         sce_forms_configuration.Machine,
		ModuleParameters: sce_forms_configuration.ModuleParameters(),
		Inputs:           sce_forms_configuration.Input,
	}
}
