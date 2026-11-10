// SCE-GENERATED — DO NOT EDIT
// source-hash: 81a3de342b33c3189f4a5813894163bc1e16bfc77aa7ead74e709e8ba7b570b9
// SCE-MAP: schema_signal.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: signal.raised
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum SchemaSignal {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "signal.raised"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 2

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 49

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:schema_signal:2"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:schema_signal:2\",\"x-sce-source-hash\":\"81a3de342b33c3189f4a5813894163bc1e16bfc77aa7ead74e709e8ba7b570b9\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"level\":{\"enum\":[\"low\",\"high\",\"critical\"]},\"count\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535},\"urgent\":{\"type\":\"boolean\"}},\"required\":[\"level\",\"count\",\"urgent\"],\"additionalProperties\":false}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct SchemaSignalPayload: Equatable, Sendable, SceDefault {
    public var level: EnumSignalLevel = EnumSignalLevel.allCases[0]
    public var count: UInt16 = 0
    public var urgent: Bool = false

    public init() {}

    public init(level: EnumSignalLevel, count: UInt16, urgent: Bool) {
        self.level = level
        self.count = count
        self.urgent = urgent
    }
}
