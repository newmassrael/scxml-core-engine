// SCE-GENERATED — DO NOT EDIT
// source-hash: 011d804f887126b888ea6ba5b4213d56e0f5c8740555a154c83e9b8d0105c4b6
// SCE-MAP: keeping_configuration.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-configuration")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package keeping_configuration

import (
	"github.com/newmassrael/sce-go-runtime/testprofile"
	sce_ptc_keeper "github.com/newmassrael/sce-go-tests/integration/static_datamodel/ptc_keeper"
	sce_idler "github.com/newmassrael/sce-go-tests/integration/static_datamodel/idler"
)

// Configuration is the test configuration `keeping_configuration` (docs/SCE_TEST_PROFILE.md
// §11.6) as the profile's runtime reads it: the MTC's component type and each PTC's, the ports
// and timers each type declares, each `<sce:connect>` and each `<sce:map>`.
func Configuration() testprofile.Configuration {
	return testprofile.Configuration{
		MTCType: "ptc_keeper",
		PTCTypes: map[string]string{
			"idler": "idler",
		},
		PortsOfType: map[string][]string{
			"idler": {"peer"},
		},
		TimersOfType: map[string][]string{
			"ptc_keeper": {"patience", "linger"},
		},
		Connections: []testprofile.Connection{},
		Mappings: []testprofile.Mapping{
			{Port: testprofile.Endpoint{Component: "idler", Port: "peer"}, SystemPort: "quiet"},
		},
	}
}

// Machine is the machine of the component type componentType of the configuration: the
// component the profile's runtime drives, generated beside its test component's machine
// (docs/SCE_TEST_PROFILE.md §12.3). Nil for a type the configuration does not name, which the
// run ends with `error` where it is created. It is a testprofile.MachineFactory.
func Machine(componentType string) testprofile.ComponentMachine {
	switch componentType {
	case "ptc_keeper":
		return &sce_ptc_keeper.PtcKeeperComponent{}
	case "idler":
		return &sce_idler.IdlerComponent{}
	}
	return nil
}

// ModuleParameters are the module parameters the components of the configuration read
// (docs/SCE_TEST_PROFILE.md §11.5), each of which a suite that runs it gives a value.
func ModuleParameters() []string {
	return []string{}
}
