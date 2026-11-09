// SCE-GENERATED — DO NOT EDIT
// source-hash: f64002805653a41c56a6a08d7b604e8955d0ac09d55e75365d49e24374faca5e
// SCE-MAP: record_announce.scxml:13 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: record_list.announce
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum RecordAnnounce {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "record_list.announce"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 31

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:record_announce:1"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:record_announce:1\",\"x-sce-source-hash\":\"f64002805653a41c56a6a08d7b604e8955d0ac09d55e75365d49e24374faca5e\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"id\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"field\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"id\",\"field\"],\"additionalProperties\":false}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct RecordAnnouncePayload: Equatable, Sendable, SceDefault {
    public var id: UInt32 = 0
    public var field: UInt16 = 0

    public init() {}

    public init(id: UInt32, field: UInt16) {
        self.id = id
        self.field = field
    }
}
