// SCE-GENERATED — DO NOT EDIT
// source-hash: 79b72bacfb4f33d945f539335f5aae4d5bf27954baed8bd3b6852401edaaac9f
// source: schema_cose_note_check.scxml
// SCE-MAP: schema_cose_note_check.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: cose.note.verify
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum SchemaCoseNoteCheck {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "cose.note.verify"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most UTF-8 bytes `trust_set` holds (the bound its type writes), as this build resolved it.
    public static let trust_setMaxSize: UInt32 = 64

    /// The most bytes `cose` holds (the bound its type writes, named `cose_signed`), as this build resolved it.
    public static let coseMaxSize: UInt32 = 4352

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 26522

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:schema_cose_note_check:1"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:schema_cose_note_check:1\",\"x-sce-source-hash\":\"79b72bacfb4f33d945f539335f5aae4d5bf27954baed8bd3b6852401edaaac9f\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"trust_set\":{\"type\":\"string\",\"maxLength\":64,\"x-sce-max-bytes\":64},\"cose\":{\"type\":\"string\",\"maxLength\":4352,\"pattern\":\"^[\\\\u0000-\\\\u00ff]*$\"}},\"required\":[\"trust_set\",\"cose\"],\"additionalProperties\":false}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order, built
/// whole by its initialiser, every field written: no type has a value nobody wrote
/// (docs/adr/0099, section 4). A value, so a host that reads one holds a copy.
public struct SchemaCoseNoteCheckPayload: Equatable, Sendable {
    public var trust_set: String
    public var cose: [UInt8]

    public init(trust_set: String, cose: [UInt8]) {
        self.trust_set = trust_set
        self.cose = cose
    }
}
