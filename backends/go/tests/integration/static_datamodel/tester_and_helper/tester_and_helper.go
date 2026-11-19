// SCE-GENERATED — DO NOT EDIT
// source-hash: 6fffef0974273f6b0d81b98b338020882f5206e2b18811e18fbb23b721e7585c
// source: tester_and_helper.scxml
// SCE-MAP: tester_and_helper.scxml:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-configuration")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package tester_and_helper

import (
	"github.com/newmassrael/sce-forge-runtime/optional"
	"github.com/newmassrael/sce-go-runtime/testprofile"
	"github.com/newmassrael/sce-go-runtime/testprofile/canonical"
	sce_job_tester "github.com/newmassrael/sce-go-tests/integration/static_datamodel/job_tester"
	sce_helper "github.com/newmassrael/sce-go-tests/integration/static_datamodel/helper"
	sce_job_status "github.com/newmassrael/sce-go-tests/integration/static_datamodel/job_status"
)

// Configuration is the test configuration `tester_and_helper` (docs/SCE_TEST_PROFILE.md
// §11.6) as the profile's runtime reads it: the MTC's component type and each PTC's, the ports
// and timers each type declares, each `<sce:connect>` and each `<sce:map>`.
func Configuration() testprofile.Configuration {
	return testprofile.Configuration{
		MTCType: "job_tester",
		PTCTypes: map[string]string{
			"helper": "helper",
		},
		PortsOfType: map[string][]string{
			"job_tester": {"sut", "ctl"},
			"helper": {"ctl"},
		},
		TimersOfType: map[string][]string{
			"job_tester": {"deadline", "settle"},
		},
		Connections: []testprofile.Connection{
			{A: testprofile.Endpoint{Component: "mtc", Port: "ctl"}, B: testprofile.Endpoint{Component: "helper", Port: "ctl"}},
		},
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
	case "job_tester":
		return &sce_job_tester.JobTesterComponent{}
	case "helper":
		return &sce_helper.HelperComponent{}
	}
	return nil
}

// ModuleParameters are the module parameters the components of the configuration read
// (docs/SCE_TEST_PROFILE.md §11.5), each of which a suite that runs it gives a value.
func ModuleParameters() []string {
	return []string{"elapsed_limit"}
}

// ReadJobStatus is the record `job_status` read from the canonical form of the test log
// (docs/SCE_TEST_PROFILE.md §12.5), as a recorded input trace holds it (docs/SCE_TTCN3_LANE.md
// §10.3), or an error naming where its value does not belong to its type. The inverse of
// CanonicalJobStatus.
func ReadJobStatus(value any) (sce_job_status.JobStatusPayload, error) {
	return func() (sce_job_status.JobStatusPayload, error) { object_0, err := canonical.ReadRecord(value, []string{"elapsed_ms", "note", "codes", "tag", "retry", "count"}, "job_status"); if err != nil { return sce_job_status.JobStatusPayload{}, err }; var read sce_job_status.JobStatusPayload; if read.ElapsedMs, err = canonical.ReadInteger[uint32](canonical.Member(object_0, "elapsed_ms"), "job_status.elapsed_ms"); err != nil { return sce_job_status.JobStatusPayload{}, err }; if read.Note, err = func(present_0 any, held bool) (optional.Of[string], error) { if !held { return optional.None[string](), nil }; read, err := canonical.ReadText(present_0, 32, "job_status.note"); if err != nil { return optional.None[string](), err }; return optional.Some(read), nil }(object_0.Get("note")); err != nil { return sce_job_status.JobStatusPayload{}, err }; if read.Codes, err = func() ([]uint8, error) { items_0, err := canonical.ReadElements(canonical.Member(object_0, "codes"), 0, 4, "job_status.codes"); if err != nil { return nil, err }; read := make([]uint8, 0, len(items_0)); for _, item_0 := range items_0 { element, err := canonical.ReadInteger[uint8](item_0, "job_status.codes[]"); if err != nil { return nil, err }; read = append(read, element) }; return read, nil }(); err != nil { return sce_job_status.JobStatusPayload{}, err }; if read.Tag, err = func(present_0 any, held bool) (optional.Of[uint8], error) { if !held { return optional.None[uint8](), nil }; read, err := canonical.ReadInteger[uint8](present_0, "job_status.tag"); if err != nil { return optional.None[uint8](), err }; return optional.Some(read), nil }(object_0.Get("tag")); err != nil { return sce_job_status.JobStatusPayload{}, err }; if read.Retry, err = func(present_0 any, held bool) (optional.Of[uint8], error) { if !held { return optional.None[uint8](), nil }; read, err := canonical.ReadInteger[uint8](present_0, "job_status.retry"); if err != nil { return optional.None[uint8](), err }; return optional.Some(read), nil }(object_0.Get("retry")); err != nil { return sce_job_status.JobStatusPayload{}, err }; if read.Count, err = canonical.ReadInteger[uint32](canonical.Member(object_0, "count"), "job_status.count"); err != nil { return sce_job_status.JobStatusPayload{}, err }; return read, nil }()
}

// CanonicalJobStatus is the record `job_status` in the canonical form of the test log
// (docs/SCE_TEST_PROFILE.md §12.5), which a message of it a trace hands the run carries.
func CanonicalJobStatus(value sce_job_status.JobStatusPayload) any {
	return canonical.Record(canonical.Field("elapsed_ms", canonical.Integer(value.ElapsedMs)), canonical.Field("note", canonical.Optional(value.Note.IsPresent(), func() any { return canonical.Text(value.Note.Proven()) })), canonical.Field("codes", canonical.List(len(value.Codes), func(item_0 int) any { return canonical.Integer(value.Codes[item_0]) })), canonical.Field("tag", canonical.Optional(value.Tag.IsPresent(), func() any { return canonical.Integer(value.Tag.Proven()) })), canonical.Field("retry", canonical.Optional(value.Retry.IsPresent(), func() any { return canonical.Integer(value.Retry.Proven()) })), canonical.Field("count", canonical.Integer(value.Count)))
}

// Input is the reader of the inputs of a recorded trace (docs/SCE_TTCN3_LANE.md §10.2, a
// testprofile.InputReader): the message an input of the record type typeName on the system port
// systemPort carries, its value read from the canonical form. An error, naming why, where the
// configuration has no such system port, the `in` list of its type does not name the type, or the
// value does not belong to it.
func Input(systemPort, typeName string, value any) (testprofile.Message, error) {
	if systemPort == "sut" && typeName == "job_status" {
		read, err := ReadJobStatus(value)
		if err != nil {
			return testprofile.Message{}, err
		}
		return testprofile.NewMessage(sce_job_status.EventName, read, "job_status", CanonicalJobStatus), nil
	}
	return testprofile.Message{}, testprofile.NotAnInput(systemPort, typeName, []string{"sut"})
}
