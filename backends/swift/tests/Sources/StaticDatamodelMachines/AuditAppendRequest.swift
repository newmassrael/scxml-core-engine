// SCE-GENERATED — DO NOT EDIT
// source-hash: 307eaeb28f847167766bd45e7895b8ba452e852e7d4ac043e80895a149da3478
// source: sce:std/host/audit_append_request.scxml
// SCE-MAP: audit_append_request.scxml:16 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: host.audit.append.request
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum AuditAppendRequest {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "host.audit.append.request"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most UTF-8 bytes `stream` holds (the bound its type writes), as this build resolved it.
    public static let streamMaxSize: UInt32 = 64

    /// The most UTF-8 bytes `entry` holds (the bound its type writes, named `audit_entry`), as this build resolved it.
    public static let entryMaxSize: UInt32 = 4096

    /// The most UTF-8 bytes `sensitivity` holds (the bound its type writes, named `audit_marks`), as this build resolved it.
    public static let sensitivityMaxSize: UInt32 = 1024

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 31145

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:audit_append_request:1"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:audit_append_request:1\",\"x-sce-source-hash\":\"307eaeb28f847167766bd45e7895b8ba452e852e7d4ac043e80895a149da3478\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"stream\":{\"type\":\"string\",\"maxLength\":64,\"x-sce-max-bytes\":64},\"entry\":{\"type\":\"string\",\"maxLength\":4096,\"x-sce-max-bytes\":4096},\"sensitivity\":{\"type\":\"string\",\"maxLength\":1024,\"x-sce-max-bytes\":1024}},\"required\":[\"stream\",\"entry\",\"sensitivity\"],\"additionalProperties\":false}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order, built
/// whole by its initialiser, every field written: no type has a value nobody wrote
/// (docs/adr/0099, section 4). A value, so a host that reads one holds a copy.
public struct AuditAppendRequestPayload: Equatable, Sendable {
    public var stream: String
    public var entry: String
    public var sensitivity: String

    public init(stream: String, entry: String, sensitivity: String) {
        self.stream = stream
        self.entry = entry
        self.sensitivity = sensitivity
    }
}
