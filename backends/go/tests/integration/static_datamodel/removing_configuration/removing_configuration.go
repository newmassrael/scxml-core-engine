// SCE-GENERATED — DO NOT EDIT
// source-hash: 982a23a5ec15b8216f29e9e8e4ba6c1047dee0d66671c7835c2776a4de5a13d2
// SCE-MAP: removing_configuration.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-configuration")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package removing_configuration

import (
	"github.com/newmassrael/sce-go-runtime/testprofile"
	sce_ptc_remover "github.com/newmassrael/sce-go-tests/integration/static_datamodel/ptc_remover"
	sce_request_listener "github.com/newmassrael/sce-go-tests/integration/static_datamodel/request_listener"
)

// Configuration is the test configuration `removing_configuration` (docs/SCE_TEST_PROFILE.md
// §11.6) as the profile's runtime reads it: the MTC's component type and each PTC's, the ports
// and timers each type declares, each `<sce:connect>` and each `<sce:map>`.
func Configuration() testprofile.Configuration {
	return testprofile.Configuration{
		MTCType: "ptc_remover",
		PTCTypes: map[string]string{
			"listener": "request_listener",
		},
		PortsOfType: map[string][]string{
			"ptc_remover": {"peer"},
			"request_listener": {"peer"},
		},
		TimersOfType: map[string][]string{
			"ptc_remover": {"patience"},
		},
		Connections: []testprofile.Connection{
			{A: testprofile.Endpoint{Component: "mtc", Port: "peer"}, B: testprofile.Endpoint{Component: "listener", Port: "peer"}},
		},
		Mappings: []testprofile.Mapping{},
	}
}

// Machine is the machine of the component type componentType of the configuration: the
// component the profile's runtime drives, generated beside its test component's machine
// (docs/SCE_TEST_PROFILE.md §12.3). Nil for a type the configuration does not name, which the
// run ends with `error` where it is created. It is a testprofile.MachineFactory.
func Machine(componentType string) testprofile.ComponentMachine {
	switch componentType {
	case "ptc_remover":
		return &sce_ptc_remover.PtcRemoverComponent{}
	case "request_listener":
		return &sce_request_listener.RequestListenerComponent{}
	}
	return nil
}

// ModuleParameters are the module parameters the components of the configuration read
// (docs/SCE_TEST_PROFILE.md §11.5), each of which a suite that runs it gives a value.
func ModuleParameters() []string {
	return []string{}
}

// Input is the reader of the inputs of a recorded trace (docs/SCE_TTCN3_LANE.md §10.2, a
// testprofile.InputReader): the message an input of the record type typeName on the system port
// systemPort carries, its value read from the canonical form. An error, naming why, where the
// configuration has no such system port, the `in` list of its type does not name the type, or the
// value does not belong to it.
func Input(systemPort, typeName string, _ any) (testprofile.Message, error) {
	return testprofile.Message{}, testprofile.NotAnInput(systemPort, typeName, []string{})
}
