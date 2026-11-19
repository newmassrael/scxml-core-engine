// SCE-GENERATED — DO NOT EDIT
// source-hash: db27c355d4b7ce1e3fd53313d6434d63dd6974c0c5b9c78426bed9ecdfb795b5
// source: unset_configuration.scxml
// SCE-MAP: unset_configuration.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-configuration")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package unset_configuration

import (
	"github.com/newmassrael/sce-go-runtime/testprofile"
	sce_no_verdict_setter "github.com/newmassrael/sce-go-tests/integration/static_datamodel/no_verdict_setter"
)

// Configuration is the test configuration `unset_configuration` (docs/SCE_TEST_PROFILE.md
// §11.6) as the profile's runtime reads it: the MTC's component type and each PTC's, the ports
// and timers each type declares, each `<sce:connect>` and each `<sce:map>`.
func Configuration() testprofile.Configuration {
	return testprofile.Configuration{
		MTCType: "no_verdict_setter",
		PTCTypes: map[string]string{},
		PortsOfType: map[string][]string{},
		TimersOfType: map[string][]string{},
		Connections: []testprofile.Connection{},
		Mappings: []testprofile.Mapping{},
	}
}

// Machine is the machine of the component type componentType of the configuration: the
// component the profile's runtime drives, generated beside its test component's machine
// (docs/SCE_TEST_PROFILE.md §12.3). Nil for a type the configuration does not name, which the
// run ends with `error` where it is created. It is a testprofile.MachineFactory.
func Machine(componentType string) testprofile.ComponentMachine {
	switch componentType {
	case "no_verdict_setter":
		return &sce_no_verdict_setter.NoVerdictSetterComponent{}
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
