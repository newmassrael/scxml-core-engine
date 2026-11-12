// SCE-GENERATED — DO NOT EDIT
// source-hash: 16d76c0f9ed42e7e13706b7e291287d0861ab6ac1014d873b4e04ff755d59e01
// SCE-MAP: keeping_configuration.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-configuration")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package keeping_configuration

import (
	"github.com/newmassrael/sce-go-runtime/testprofile"
	"github.com/newmassrael/sce-go-runtime/testprofile/canonical"
	sce_ptc_keeper "github.com/newmassrael/sce-go-tests/integration/static_datamodel/ptc_keeper"
	sce_idler "github.com/newmassrael/sce-go-tests/integration/static_datamodel/idler"
	sce_job_reply "github.com/newmassrael/sce-go-tests/integration/static_datamodel/job_reply"
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

// ReadJobReply is the record `job_reply` read from the canonical form of the test log
// (docs/SCE_TEST_PROFILE.md §12.5), as a recorded input trace holds it (docs/SCE_TTCN3_LANE.md
// §10.3), or an error naming where its value does not belong to its type. The inverse of
// CanonicalJobReply.
func ReadJobReply(value any) (sce_job_reply.JobReplyPayload, error) {
	return func() (sce_job_reply.JobReplyPayload, error) { object_0, err := canonical.ReadRecord(value, []string{"job", "code"}, "job_reply"); if err != nil { return sce_job_reply.JobReplyPayload{}, err }; var read sce_job_reply.JobReplyPayload; if read.Job, err = canonical.ReadInteger[uint32](canonical.Member(object_0, "job"), "job_reply.job"); err != nil { return sce_job_reply.JobReplyPayload{}, err }; if read.Code, err = canonical.ReadInteger[uint32](canonical.Member(object_0, "code"), "job_reply.code"); err != nil { return sce_job_reply.JobReplyPayload{}, err }; return read, nil }()
}

// CanonicalJobReply is the record `job_reply` in the canonical form of the test log
// (docs/SCE_TEST_PROFILE.md §12.5), which a message of it a trace hands the run carries.
func CanonicalJobReply(value sce_job_reply.JobReplyPayload) any {
	return canonical.Record(canonical.Field("job", canonical.Integer(value.Job)), canonical.Field("code", canonical.Integer(value.Code)))
}

// Input is the reader of the inputs of a recorded trace (docs/SCE_TTCN3_LANE.md §10.2, a
// testprofile.InputReader): the message an input of the record type typeName on the system port
// systemPort carries, its value read from the canonical form. An error, naming why, where the
// configuration has no such system port, the `in` list of its type does not name the type, or the
// value does not belong to it.
func Input(systemPort, typeName string, value any) (testprofile.Message, error) {
	if systemPort == "quiet" && typeName == "job_reply" {
		read, err := ReadJobReply(value)
		if err != nil {
			return testprofile.Message{}, err
		}
		return testprofile.NewMessage(sce_job_reply.EventName, read, "job_reply", CanonicalJobReply), nil
	}
	return testprofile.Message{}, testprofile.NotAnInput(systemPort, typeName, []string{"quiet"})
}
