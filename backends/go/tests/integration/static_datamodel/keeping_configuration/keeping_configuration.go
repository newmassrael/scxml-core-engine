// SCE-GENERATED — DO NOT EDIT
// source-hash: 021fdf0d0c3ee151c664e2ecf6a7e5a797d68aaf3d399831e8dfd49b9e10bec9
// SCE-MAP: keeping_configuration.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-configuration")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package keeping_configuration

import (
	"github.com/newmassrael/sce-go-runtime/testprofile"
	sce_keeper "github.com/newmassrael/sce-go-tests/integration/static_datamodel/keeper"
	sce_idler "github.com/newmassrael/sce-go-tests/integration/static_datamodel/idler"
)

// Configuration is the test configuration `keeping_configuration` (docs/SCE_TEST_PROFILE.md
// §11.6) as the profile's runtime reads it: the MTC's component type and each PTC's, the ports
// and timers each type declares, each `<sce:connect>` and each `<sce:map>`.
func Configuration() testprofile.Configuration {
	return testprofile.Configuration{
		MTCType: "keeper",
		PTCTypes: map[string]string{
			"idler": "idler",
		},
		PortsOfType: map[string][]string{
			"idler": {"peer"},
		},
		TimersOfType: map[string][]string{
			"keeper": {"patience", "linger"},
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
	case "keeper":
		return &sce_keeper.KeeperComponent{}
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
