// SCE-GENERATED — DO NOT EDIT
// source-hash: c4657e911a812ec796ce0b3c5be3eeef2b66326118a66c11d56342e555c6a85c
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
    public static let jsonSchema: String = "{\"$id\":\"urn:sce:schema:schema_signal:2\",\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"additionalProperties\":false,\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"properties\":{\"count\":{\"maximum\":65535,\"minimum\":0,\"type\":\"integer\"},\"level\":{\"enum\":[\"low\",\"high\",\"critical\"]},\"urgent\":{\"type\":\"boolean\"}},\"required\":[\"count\",\"level\",\"urgent\"],\"type\":\"object\",\"x-sce-source-hash\":\"c4657e911a812ec796ce0b3c5be3eeef2b66326118a66c11d56342e555c6a85c\"}"
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
