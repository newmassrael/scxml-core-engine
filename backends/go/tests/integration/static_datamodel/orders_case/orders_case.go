// SCE-GENERATED — DO NOT EDIT
// source-hash: be94512ba39db699a331b27cc31692bda58f20eb3a2d7dac3ed4ad84dab78fc5
// source: orders_case.scxml
// SCE-MAP: orders_case.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-case")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package orders_case

import (
	"github.com/newmassrael/sce-go-runtime/testprofile"
	sce_orders_configuration "github.com/newmassrael/sce-go-tests/integration/static_datamodel/orders_configuration"
)

// Name is the test case's name in the suite and in the test log (docs/SCE_TEST_PROFILE.md §11.7).
const Name string = "orders_case"

// TestCase is the test case `orders_case` (docs/SCE_TEST_PROFILE.md §11.7) as the runner
// reads it: its name, its configuration and the machines of its components, the module
// parameters those components read, and the reader of the inputs a recorded trace of it carries
// (docs/SCE_TTCN3_LANE.md §10.5).
func TestCase() testprofile.TestCase {
	return testprofile.TestCase{
		Name:             Name,
		Configuration:    sce_orders_configuration.Configuration(),
		Machines:         sce_orders_configuration.Machine,
		ModuleParameters: sce_orders_configuration.ModuleParameters(),
		Inputs:           sce_orders_configuration.Input,
	}
}
