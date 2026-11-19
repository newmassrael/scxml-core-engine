// SCE-GENERATED — DO NOT EDIT
// source-hash: 0c26ac0a658b0d22855ffe96d3d8f7019bbe1d61ce11af78d39fe2b89dcbcfa4
// source: schema_server_limits.scxml
// SCE-MAP: schema_server_limits.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: server.limits
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum SchemaServerLimits {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "server.limits"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most UTF-8 bytes `label` holds (the bound its type writes), as this build resolved it.
    public static let labelMaxSize: UInt32 = 16

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 130

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:schema_server_limits:1"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:schema_server_limits:1\",\"x-sce-source-hash\":\"0c26ac0a658b0d22855ffe96d3d8f7019bbe1d61ce11af78d39fe2b89dcbcfa4\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"max_rows\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"label\":{\"type\":\"string\",\"maxLength\":16,\"x-sce-max-bytes\":16}},\"required\":[\"max_rows\",\"label\"],\"additionalProperties\":false}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order, built
/// whole by its initialiser, every field written: no type has a value nobody wrote
/// (docs/adr/0099, section 4). A value, so a host that reads one holds a copy.
public struct SchemaServerLimitsPayload: Equatable, Sendable {
    public var max_rows: UInt32
    public var label: String

    public init(max_rows: UInt32, label: String) {
        self.max_rows = max_rows
        self.label = label
    }
}
