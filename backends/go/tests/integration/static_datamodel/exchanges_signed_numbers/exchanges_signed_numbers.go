// SCE-GENERATED — DO NOT EDIT
// source-hash: f5971bce067d83cde6ccc31d05292c7b14a559e06de0ad63b783e80863971fec
// source: exchanges_signed_numbers.scxml
// SCE-MAP: exchanges_signed_numbers.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-case")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package exchanges_signed_numbers

import (
	"github.com/newmassrael/sce-go-runtime/testprofile"
	sce_signed_configuration "github.com/newmassrael/sce-go-tests/integration/static_datamodel/signed_configuration"
)

// Name is the test case's name in the suite and in the test log (docs/SCE_TEST_PROFILE.md §11.7).
const Name string = "exchanges_signed_numbers"

// TestCase is the test case `exchanges_signed_numbers` (docs/SCE_TEST_PROFILE.md §11.7) as the runner
// reads it: its name, its configuration and the machines of its components, the module
// parameters those components read, and the reader of the inputs a recorded trace of it carries
// (docs/SCE_TTCN3_LANE.md §10.5).
func TestCase() testprofile.TestCase {
	return testprofile.TestCase{
		Name:             Name,
		Configuration:    sce_signed_configuration.Configuration(),
		Machines:         sce_signed_configuration.Machine,
		ModuleParameters: sce_signed_configuration.ModuleParameters(),
		Inputs:           sce_signed_configuration.Input,
	}
}
