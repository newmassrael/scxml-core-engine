// SCE-GENERATED — DO NOT EDIT
// source-hash: 50ba4afa70e08ed0ad002509e6baae9623e17ddc8afb219bbba6da905f187359
// SCE-MAP: schema_day.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: day.picked
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum SchemaDay {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "day.picked"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 43

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:schema_day:1"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:schema_day:1\",\"x-sce-source-hash\":\"50ba4afa70e08ed0ad002509e6baae9623e17ddc8afb219bbba6da905f187359\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"year\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535},\"month\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"dayOfMonth\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255}},\"required\":[\"year\",\"month\",\"dayOfMonth\"],\"additionalProperties\":false}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct SchemaDayPayload: Equatable, Sendable, SceDefault {
    public var year: UInt16 = 0
    public var month: UInt8 = 0
    public var dayOfMonth: UInt8 = 0

    public init() {}

    public init(year: UInt16, month: UInt8, dayOfMonth: UInt8) {
        self.year = year
        self.month = month
        self.dayOfMonth = dayOfMonth
    }
}
