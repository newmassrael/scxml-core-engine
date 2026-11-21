// SCE-GENERATED — DO NOT EDIT
// source-hash: d838a7c88c0c55aed8e20d4ea0997c27476c45c92103d5589f0b3083fa0f5949
// source: schema_cose_note.scxml
// SCE-MAP: schema_cose_note.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: cose.note.sign
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum SchemaCoseNote {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "cose.note.sign"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most UTF-8 bytes `owner` holds (the bound its type writes), as this build resolved it.
    public static let ownerMaxSize: UInt32 = 24

    /// The most bytes `blob` holds (the bound its type writes), as this build resolved it.
    public static let blobMaxSize: UInt32 = 8

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 265

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:schema_cose_note:1"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:schema_cose_note:1\",\"x-sce-source-hash\":\"d838a7c88c0c55aed8e20d4ea0997c27476c45c92103d5589f0b3083fa0f5949\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"serial\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"owner\":{\"type\":\"string\",\"maxLength\":24,\"x-sce-max-bytes\":24},\"blob\":{\"type\":\"string\",\"maxLength\":8,\"pattern\":\"^[\\\\u0000-\\\\u00ff]*$\"},\"active\":{\"type\":\"boolean\"},\"offset\":{\"type\":\"integer\",\"minimum\":-32768,\"maximum\":32767}},\"required\":[\"serial\",\"owner\",\"blob\",\"active\",\"offset\"],\"additionalProperties\":false}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order, built
/// whole by its initialiser, every field written: no type has a value nobody wrote
/// (docs/adr/0099, section 4). A value, so a host that reads one holds a copy.
public struct SchemaCoseNotePayload: Equatable, Sendable {
    public var serial: UInt32
    public var owner: String
    public var blob: [UInt8]
    public var active: Bool
    public var offset: Int16

    public init(serial: UInt32, owner: String, blob: [UInt8], active: Bool, offset: Int16) {
        self.serial = serial
        self.owner = owner
        self.blob = blob
        self.active = active
        self.offset = offset
    }
}
