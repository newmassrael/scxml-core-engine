// SCE-GENERATED — DO NOT EDIT
// source-hash: ea764ab5524bc7a7e7d4553640b3fb26547d13f0d2053c0f5e7214161efbc6d0
// SCE-MAP: schema_revised.scxml:15 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: reading.revised
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum SchemaRevised {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "reading.revised"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 3

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 51

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:schema_revised:3"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:schema_revised:3\",\"x-sce-source-hash\":\"ea764ab5524bc7a7e7d4553640b3fb26547d13f0d2053c0f5e7214161efbc6d0\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"level\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"layout\":{\"enum\":[\"month\",\"week\",\"day\",\"agenda_list\"]},\"urgent\":{\"type\":\"boolean\"}},\"required\":[\"level\",\"layout\",\"urgent\"],\"additionalProperties\":false}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct SchemaRevisedPayload: Equatable, Sendable, SceDefault {
    public var level: UInt8 = 0
    public var layout: EnumViewMode = EnumViewMode.allCases[0]
    public var urgent: Bool = false

    public init() {}

    public init(level: UInt8, layout: EnumViewMode, urgent: Bool) {
        self.level = level
        self.layout = layout
        self.urgent = urgent
    }
}
