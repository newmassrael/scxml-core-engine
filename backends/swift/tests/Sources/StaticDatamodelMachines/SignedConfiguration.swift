// SCE-GENERATED — DO NOT EDIT
// source-hash: 982a23a5ec15b8216f29e9e8e4ba6c1047dee0d66671c7835c2776a4de5a13d2
// SCE-MAP: signed_configuration.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-configuration")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

import SceRuntime
import SceTestProfile

/// The test configuration `signed_configuration` (docs/SCE_TEST_PROFILE.md §11.6) as the
/// profile's runtime reads it, the members of a caseless enum of the module as every unit's are:
/// the configuration, the factory of its components' machines, the module parameters they read,
/// and the reader of the inputs a recorded trace carries on its system ports
/// (docs/SCE_TTCN3_LANE.md §10.5).
public enum SignedConfiguration {
    /// The configuration: the MTC's component type and each PTC's, the ports and timers each type
    /// declares, each `<sce:connect>` and each `<sce:map>`.
    public static func configuration() -> SceTestProfile.Configuration {
        SceTestProfile.Configuration(
            mtcType: "signed_tester",
            ptcTypes: [:],
            portsOfType: [
                "signed_tester": ["sut"],
            ],
            timersOfType: [
                "signed_tester": ["deadline"],
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
        case "signed_tester": return SignedTesterComponent()
        default: return nil
        }
    }

    /// The module parameters the components of the configuration read (docs/SCE_TEST_PROFILE.md
    /// §11.5), each of which a suite that runs it gives a value.
    public static func moduleParameters() -> [String] {
        []
    }

    /// The record `signed_reading` read from the canonical form of the test log
    /// (docs/SCE_TEST_PROFILE.md §12.5), as a recorded input trace holds it
    /// (docs/SCE_TTCN3_LANE.md §10.3). The inverse of `canonical_signed_reading`.
    ///
    /// - Throws: `SceTestProfile.TraceRefusal` naming where its value does not belong to its type.
    public static func read_signed_reading(_ value: SceRuntime.JSONValue) throws(SceTestProfile.TraceRefusal)
        -> SignedReadingPayload
    {
        try SceTestProfile.FromCanonical.record(value, ["delta", "offset", "level", "stamp", "total"], "signed_reading") { (record_0: SceRuntime.JSONValue) throws(SceTestProfile.TraceRefusal) -> SignedReadingPayload in try SignedReadingPayload(delta: SceTestProfile.FromCanonical.integer(SceTestProfile.FromCanonical.field(record_0, "delta", "signed_reading.delta"), Int8.self, "signed_reading.delta"), offset: SceTestProfile.FromCanonical.integer(SceTestProfile.FromCanonical.field(record_0, "offset", "signed_reading.offset"), Int16.self, "signed_reading.offset"), level: SceTestProfile.FromCanonical.integer(SceTestProfile.FromCanonical.field(record_0, "level", "signed_reading.level"), Int32.self, "signed_reading.level"), stamp: SceTestProfile.FromCanonical.integer(SceTestProfile.FromCanonical.field(record_0, "stamp", "signed_reading.stamp"), Int64.self, "signed_reading.stamp"), total: SceTestProfile.FromCanonical.integer(SceTestProfile.FromCanonical.field(record_0, "total", "signed_reading.total"), UInt64.self, "signed_reading.total")) }
    }

    /// The record `signed_reading` in the canonical form of the test log
    /// (docs/SCE_TEST_PROFILE.md §12.5), which a message of it a trace hands the run carries.
    public static func canonical_signed_reading(_ value: SignedReadingPayload) -> SceRuntime.JSONValue {
        SceTestProfile.Canonical.record(("delta", SceTestProfile.Canonical.integer(value.delta)), ("offset", SceTestProfile.Canonical.integer(value.offset)), ("level", SceTestProfile.Canonical.integer(value.level)), ("stamp", SceTestProfile.Canonical.integer(value.stamp)), ("total", SceTestProfile.Canonical.integer(value.total)))
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
        case ("sut", "signed_reading"):
            return try SceTestProfile.Message.of(
                event: SignedReading.eventName, value: read_signed_reading(value), typeName: "signed_reading",
                canonical: { SignedConfiguration.canonical_signed_reading($0) })
        default:
            throw SceTestProfile.TraceRefusal.notAnInput(systemPort, typeName, ["sut"])
        }
    }
}
