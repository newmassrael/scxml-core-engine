// SCE-GENERATED — DO NOT EDIT
// source-hash: 7b98612ed0f35af10eaffafb7dd60088b78d6093971dc622b4d3161a336da46c
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

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 53

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:event_schema_sensor_sample:1"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:event_schema_sensor_sample:1\",\"x-sce-source-hash\":\"7b98612ed0f35af10eaffafb7dd60088b78d6093971dc622b4d3161a336da46c\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"sensorId\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535},\"reading\":{\"type\":\"integer\",\"minimum\":-2147483648,\"maximum\":2147483647},\"status\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255}},\"required\":[\"sensorId\",\"reading\",\"status\"],\"additionalProperties\":false}"
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
