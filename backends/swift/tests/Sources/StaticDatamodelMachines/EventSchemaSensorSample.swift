// SCE-GENERATED — DO NOT EDIT
// source-hash: 6b431984688d288798089f16432b8bdf8a4e08b28da85ae7117e434e9ad27889
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
