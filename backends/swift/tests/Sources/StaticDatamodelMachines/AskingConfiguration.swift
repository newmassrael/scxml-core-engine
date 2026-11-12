// SCE-GENERATED — DO NOT EDIT
// source-hash: 16d76c0f9ed42e7e13706b7e291287d0861ab6ac1014d873b4e04ff755d59e01
// SCE-MAP: asking_configuration.scxml:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-configuration")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

import SceRuntime
import SceTestProfile

/// The test configuration `asking_configuration` (docs/SCE_TEST_PROFILE.md §11.6) as the
/// profile's runtime reads it, the members of a caseless enum of the module as every unit's are:
/// the configuration, the factory of its components' machines, the module parameters they read,
/// and the reader of the inputs a recorded trace carries on its system ports
/// (docs/SCE_TTCN3_LANE.md §10.5).
public enum AskingConfiguration {
    /// The configuration: the MTC's component type and each PTC's, the ports and timers each type
    /// declares, each `<sce:connect>` and each `<sce:map>`.
    public static func configuration() -> SceTestProfile.Configuration {
        SceTestProfile.Configuration(
            mtcType: "param_asker",
            ptcTypes: [:],
            portsOfType: [
                "param_asker": ["sut"],
            ],
            timersOfType: [
                "param_asker": ["deadline"],
            ],
            connections: [],
            mappings: [
                SceTestProfile.Mapping(SceTestProfile.Endpoint(component: "mtc", port: "sut"), systemPort: "sut"),
            ])
    }

    /// The machine of each component type of the configuration: the component the profile's
    /// runtime drives, generated beside its test component's machine (docs/SCE_TEST_PROFILE.md
    /// §12.3). Nil for a type the configuration does not name, which the run ends with `error`
    /// where it is created.
    public static func machine(_ componentType: String) -> (any SceTestProfile.ComponentMachine)? {
        switch componentType {
        case "param_asker": return ParamAskerComponent()
        default: return nil
        }
    }

    /// The module parameters the components of the configuration read (docs/SCE_TEST_PROFILE.md
    /// §11.5), each of which a suite that runs it gives a value.
    public static func moduleParameters() -> [String] {
        ["job"]
    }

    /// The record `job_reply` read from the canonical form of the test log
    /// (docs/SCE_TEST_PROFILE.md §12.5), as a recorded input trace holds it
    /// (docs/SCE_TTCN3_LANE.md §10.3). The inverse of `canonical_job_reply`.
    ///
    /// - Throws: `SceTestProfile.TraceRefusal` naming where its value does not belong to its type.
    public static func read_job_reply(_ value: SceRuntime.JSONValue) throws(SceTestProfile.TraceRefusal)
        -> JobReplyPayload
    {
        try SceTestProfile.FromCanonical.record(value, ["job", "code"], "job_reply") { (record_0: SceRuntime.JSONValue) throws(SceTestProfile.TraceRefusal) -> JobReplyPayload in try JobReplyPayload(job: SceTestProfile.FromCanonical.integer(SceTestProfile.FromCanonical.field(record_0, "job", "job_reply.job"), UInt32.self, "job_reply.job"), code: SceTestProfile.FromCanonical.integer(SceTestProfile.FromCanonical.field(record_0, "code", "job_reply.code"), UInt32.self, "job_reply.code")) }
    }

    /// The record `job_reply` in the canonical form of the test log
    /// (docs/SCE_TEST_PROFILE.md §12.5), which a message of it a trace hands the run carries.
    public static func canonical_job_reply(_ value: JobReplyPayload) -> SceRuntime.JSONValue {
        SceTestProfile.Canonical.record(("job", SceTestProfile.Canonical.integer(value.job)), ("code", SceTestProfile.Canonical.integer(value.code)))
    }

    /// The reader of the inputs of a recorded trace (docs/SCE_TTCN3_LANE.md §10.2,
    /// `SceTestProfile.InputReader`): the message an input of the record type `typeName` on the
    /// system port `systemPort` carries, its value read from the canonical form.
    ///
    /// - Throws: `SceTestProfile.TraceRefusal` naming why, where the configuration has no such system
    ///   port, the `in` list of its type does not name the type, or the value does not belong to it.
    public static func input(systemPort: String, typeName: String, value: SceRuntime.JSONValue)
        throws(SceTestProfile.TraceRefusal) -> SceTestProfile.Message
    {
        switch (systemPort, typeName) {
        case ("sut", "job_reply"):
            return try SceTestProfile.Message.of(
                event: JobReply.eventName, value: read_job_reply(value), typeName: "job_reply",
                canonical: { AskingConfiguration.canonical_job_reply($0) })
        default:
            throw SceTestProfile.TraceRefusal.notAnInput(systemPort, typeName, ["sut"])
        }
    }
}
