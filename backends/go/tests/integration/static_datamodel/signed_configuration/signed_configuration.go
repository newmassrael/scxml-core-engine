// SCE-GENERATED — DO NOT EDIT
// source-hash: 2b1a01faa0f39a78981b432fbb0fd087ee06ecdcae837825ff5c891fee416aff
// source: signed_configuration.scxml
// SCE-MAP: signed_configuration.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-configuration")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package signed_configuration

import (
	"github.com/newmassrael/sce-go-runtime/testprofile"
	"github.com/newmassrael/sce-go-runtime/testprofile/canonical"
	sce_signed_tester "github.com/newmassrael/sce-go-tests/integration/static_datamodel/signed_tester"
	sce_signed_reading "github.com/newmassrael/sce-go-tests/integration/static_datamodel/signed_reading"
)

// Configuration is the test configuration `signed_configuration` (docs/SCE_TEST_PROFILE.md
// §11.6) as the profile's runtime reads it: the MTC's component type and each PTC's, the ports
// and timers each type declares, each `<sce:connect>` and each `<sce:map>`.
func Configuration() testprofile.Configuration {
	return testprofile.Configuration{
		MTCType: "signed_tester",
		PTCTypes: map[string]string{},
		PortsOfType: map[string][]string{
			"signed_tester": {"sut"},
		},
		TimersOfType: map[string][]string{
			"signed_tester": {"deadline"},
		},
		Connections: []testprofile.Connection{},
		Mappings: []testprofile.Mapping{
			{Port: testprofile.Endpoint{Component: "mtc", Port: "sut"}, SystemPort: "sut"},
		},
	}
}

// Machine is the machine of the component type componentType of the configuration: the
// component the profile's runtime drives, generated beside its test component's machine
// (docs/SCE_TEST_PROFILE.md §12.3). Nil for a type the configuration does not name, which the
// run ends with `error` where it is created. It is a testprofile.MachineFactory.
func Machine(componentType string) testprofile.ComponentMachine {
	switch componentType {
	case "signed_tester":
		return &sce_signed_tester.SignedTesterComponent{}
	}
	return nil
}

// ModuleParameters are the module parameters the components of the configuration read
// (docs/SCE_TEST_PROFILE.md §11.5), each of which a suite that runs it gives a value.
func ModuleParameters() []string {
	return []string{}
}

// ReadSignedReading is the record `signed_reading` read from the canonical form of the test log
// (docs/SCE_TEST_PROFILE.md §12.5), as a recorded input trace holds it (docs/SCE_TTCN3_LANE.md
// §10.3), or an error naming where its value does not belong to its type. The inverse of
// CanonicalSignedReading.
func ReadSignedReading(value any) (sce_signed_reading.SignedReadingPayload, error) {
	return func() (sce_signed_reading.SignedReadingPayload, error) { object_0, err := canonical.ReadRecord(value, []string{"change", "offset", "level", "stamp", "total"}, "signed_reading"); if err != nil { return sce_signed_reading.SignedReadingPayload{}, err }; var read sce_signed_reading.SignedReadingPayload; if read.Change, err = canonical.ReadInteger[int8](canonical.Member(object_0, "change"), "signed_reading.change"); err != nil { return sce_signed_reading.SignedReadingPayload{}, err }; if read.Offset, err = canonical.ReadInteger[int16](canonical.Member(object_0, "offset"), "signed_reading.offset"); err != nil { return sce_signed_reading.SignedReadingPayload{}, err }; if read.Level, err = canonical.ReadInteger[int32](canonical.Member(object_0, "level"), "signed_reading.level"); err != nil { return sce_signed_reading.SignedReadingPayload{}, err }; if read.Stamp, err = canonical.ReadInteger[int64](canonical.Member(object_0, "stamp"), "signed_reading.stamp"); err != nil { return sce_signed_reading.SignedReadingPayload{}, err }; if read.Total, err = canonical.ReadInteger[uint64](canonical.Member(object_0, "total"), "signed_reading.total"); err != nil { return sce_signed_reading.SignedReadingPayload{}, err }; return read, nil }()
}

// CanonicalSignedReading is the record `signed_reading` in the canonical form of the test log
// (docs/SCE_TEST_PROFILE.md §12.5), which a message of it a trace hands the run carries.
func CanonicalSignedReading(value sce_signed_reading.SignedReadingPayload) any {
	return canonical.Record(canonical.Field("change", canonical.Integer(value.Change)), canonical.Field("offset", canonical.Integer(value.Offset)), canonical.Field("level", canonical.Integer(value.Level)), canonical.Field("stamp", canonical.Integer(value.Stamp)), canonical.Field("total", canonical.Integer(value.Total)))
}

// Input is the reader of the inputs of a recorded trace (docs/SCE_TTCN3_LANE.md §10.2, a
// testprofile.InputReader): the message an input of the record type typeName on the system port
// systemPort carries, its value read from the canonical form. An error, naming why, where the
// configuration has no such system port, the `in` list of its type does not name the type, or the
// value does not belong to it.
func Input(systemPort, typeName string, value any) (testprofile.Message, error) {
	if systemPort == "sut" && typeName == "signed_reading" {
		read, err := ReadSignedReading(value)
		if err != nil {
			return testprofile.Message{}, err
		}
		return testprofile.NewMessage(sce_signed_reading.EventName, read, "signed_reading", CanonicalSignedReading), nil
	}
	return testprofile.Message{}, testprofile.NotAnInput(systemPort, typeName, []string{"sut"})
}
