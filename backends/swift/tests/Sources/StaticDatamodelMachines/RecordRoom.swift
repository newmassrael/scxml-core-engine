// SCE-GENERATED — DO NOT EDIT
// source-hash: 81cc59c6b8f46736a80b4b69ee489274d9b0ae32efae821805ba44c3bac3af2c
// SCE-MAP: record_room.scxml:27 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: record_list.room
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum RecordRoom {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "record_list.room"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 62

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:record_room:1"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:record_room:1\",\"x-sce-source-hash\":\"81cc59c6b8f46736a80b4b69ee489274d9b0ae32efae821805ba44c3bac3af2c\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"values\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"history\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"digest\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295}},\"required\":[\"values\",\"history\",\"digest\"],\"additionalProperties\":false}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order, built
/// whole by its initialiser, every field written: no type has a value nobody wrote
/// (docs/adr/0099, section 4). A value, so a host that reads one holds a copy.
public struct RecordRoomPayload: Equatable, Sendable {
    public var values: UInt32
    public var history: UInt32
    public var digest: UInt32

    public init(values: UInt32, history: UInt32, digest: UInt32) {
        self.values = values
        self.history = history
        self.digest = digest
    }
}
