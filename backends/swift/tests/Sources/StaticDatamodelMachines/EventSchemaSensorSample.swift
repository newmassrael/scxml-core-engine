// SCE-GENERATED — DO NOT EDIT
// source-hash: 90dfacafb9c3b3dd4ea571704b12a814c54947805373c06b331ad43ea5f220d0
// SCE-MAP: event_schema_sensor_sample.scxml:24 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: sensor.sample
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum EventSchemaSensorSample {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "sensor.sample"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct EventSchemaSensorSamplePayload: Equatable, Sendable, SceDefault {
    public var sensorId: UInt16 = 0
    public var reading: Int32 = 0
    public var status: UInt8 = 0

    public init() {}

    public init(sensorId: UInt16, reading: Int32, status: UInt8) {
        self.sensorId = sensorId
        self.reading = reading
        self.status = status
    }
}
