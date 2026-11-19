// SCE-GENERATED — DO NOT EDIT
// source-hash: cf2440c31d1d10f13e20f63c1fa6e96557f4668f76f8e9c9e4cf7c9070518911
// source: sce:std/host/random_bytes_request.scxml
// SCE-MAP: random_bytes_request.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: host.random.bytes.request
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum RandomBytesRequest {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "host.random.bytes.request"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 16

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:random_bytes_request:1"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:random_bytes_request:1\",\"x-sce-source-hash\":\"cf2440c31d1d10f13e20f63c1fa6e96557f4668f76f8e9c9e4cf7c9070518911\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"length\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"length\"],\"additionalProperties\":false}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order, built
/// whole by its initialiser, every field written: no type has a value nobody wrote
/// (docs/adr/0099, section 4). A value, so a host that reads one holds a copy.
public struct RandomBytesRequestPayload: Equatable, Sendable {
    public var length: UInt16

    public init(length: UInt16) {
        self.length = length
    }
}
