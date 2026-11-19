// SCE-GENERATED — DO NOT EDIT
// source-hash: 6fffef0974273f6b0d81b98b338020882f5206e2b18811e18fbb23b721e7585c
// source: tester_and_helper.scxml
// SCE-MAP: tester_and_helper.scxml:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-configuration")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

import SceRuntime
import SceTestProfile

/// The test configuration `tester_and_helper` (docs/SCE_TEST_PROFILE.md §11.6) as the
/// profile's runtime reads it, the members of a caseless enum of the module as every unit's are:
/// the configuration, the factory of its components' machines, the module parameters they read,
/// and the reader of the inputs a recorded trace carries on its system ports
/// (docs/SCE_TTCN3_LANE.md §10.5).
public enum TesterAndHelper {
    /// The configuration: the MTC's component type and each PTC's, the ports and timers each type
    /// declares, each `<sce:connect>` and each `<sce:map>`.
    public static func configuration() -> SceTestProfile.Configuration {
        SceTestProfile.Configuration(
            mtcType: "job_tester",
            ptcTypes: [
                "helper": "helper",
            ],
            portsOfType: [
                "job_tester": ["sut", "ctl"],
                "helper": ["ctl"],
            ],
            timersOfType: [
                "job_tester": ["deadline", "settle"],
            ],
            connections: [
                SceTestProfile.Connection(SceTestProfile.Endpoint(component: "mtc", port: "ctl"), SceTestProfile.Endpoint(component: "helper", port: "ctl")),
            ],
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
        case "job_tester": return JobTesterComponent()
        case "helper": return HelperComponent()
        default: return nil
        }
    }

    /// The module parameters the components of the configuration read (docs/SCE_TEST_PROFILE.md
    /// §11.5), each of which a suite that runs it gives a value.
    public static func moduleParameters() -> [String] {
        ["elapsed_limit"]
    }

    /// The record `job_status` read from the canonical form of the test log
    /// (docs/SCE_TEST_PROFILE.md §12.5), as a recorded input trace holds it
    /// (docs/SCE_TTCN3_LANE.md §10.3). The inverse of `canonical_job_status`.
    ///
    /// - Throws: `SceTestProfile.TraceRefusal` naming where its value does not belong to its type.
    public static func read_job_status(_ value: SceRuntime.JSONValue) throws(SceTestProfile.TraceRefusal)
        -> JobStatusPayload
    {
        try SceTestProfile.FromCanonical.record(value, ["elapsed_ms", "note", "codes", "tag", "retry", "count"], "job_status") { (record_0: SceRuntime.JSONValue) throws(SceTestProfile.TraceRefusal) -> JobStatusPayload in try JobStatusPayload(elapsed_ms: SceTestProfile.FromCanonical.integer(SceTestProfile.FromCanonical.field(record_0, "elapsed_ms", "job_status.elapsed_ms"), UInt32.self, "job_status.elapsed_ms"), note: SceTestProfile.FromCanonical.optionalField(record_0, "note") { present_0 throws(SceTestProfile.TraceRefusal) in try SceTestProfile.FromCanonical.text(present_0, 32, "job_status.note") }, codes: SceTestProfile.FromCanonical.elements(SceTestProfile.FromCanonical.field(record_0, "codes", "job_status.codes"), 0, 4, "job_status.codes") { item_0 throws(SceTestProfile.TraceRefusal) in try SceTestProfile.FromCanonical.integer(item_0, UInt8.self, "job_status.codes[]") }, tag: SceTestProfile.FromCanonical.optionalField(record_0, "tag") { present_0 throws(SceTestProfile.TraceRefusal) in try SceTestProfile.FromCanonical.integer(present_0, UInt8.self, "job_status.tag") }, retry: SceTestProfile.FromCanonical.optionalField(record_0, "retry") { present_0 throws(SceTestProfile.TraceRefusal) in try SceTestProfile.FromCanonical.integer(present_0, UInt8.self, "job_status.retry") }, count: SceTestProfile.FromCanonical.integer(SceTestProfile.FromCanonical.field(record_0, "count", "job_status.count"), UInt32.self, "job_status.count")) }
    }

    /// The record `job_status` in the canonical form of the test log
    /// (docs/SCE_TEST_PROFILE.md §12.5), which a message of it a trace hands the run carries.
    public static func canonical_job_status(_ value: JobStatusPayload) -> SceRuntime.JSONValue {
        SceTestProfile.Canonical.record(("elapsed_ms", SceTestProfile.Canonical.integer(value.elapsed_ms)), ("note", value.note.map { present_0 -> SceRuntime.JSONValue in SceTestProfile.Canonical.text(present_0) }), ("codes", SceTestProfile.Canonical.list(value.codes.map { item_0 -> SceRuntime.JSONValue in SceTestProfile.Canonical.integer(item_0) })), ("tag", value.tag.map { present_0 -> SceRuntime.JSONValue in SceTestProfile.Canonical.integer(present_0) }), ("retry", value.retry.map { present_0 -> SceRuntime.JSONValue in SceTestProfile.Canonical.integer(present_0) }), ("count", SceTestProfile.Canonical.integer(value.count)))
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
        case ("sut", "job_status"):
            return try SceTestProfile.Message.of(
                event: JobStatus.eventName, value: read_job_status(value), typeName: "job_status",
                canonical: { TesterAndHelper.canonical_job_status($0) })
        default:
            throw SceTestProfile.TraceRefusal.notAnInput(systemPort, typeName, ["sut"])
        }
    }
}
