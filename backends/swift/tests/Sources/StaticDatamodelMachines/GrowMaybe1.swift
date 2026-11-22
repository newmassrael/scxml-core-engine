// SCE-GENERATED — DO NOT EDIT
// source-hash: 75ff8a217f4bbd86f0ed3149c6c9f6f26f6413a55e448662e67c1f3f0faa3182
// source: grow_maybe_1.scxml
// SCE-MAP: grow_maybe_1.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: grow.maybe
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum GrowMaybe1 {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "grow.maybe"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 21

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:grow_maybe_1:1"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:grow_maybe_1:1\",\"x-sce-source-hash\":\"75ff8a217f4bbd86f0ed3149c6c9f6f26f6413a55e448662e67c1f3f0faa3182\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"inner\":{\"anyOf\":[{\"$ref\":\"#/$defs/grow_inner_1\"},{\"type\":\"null\"}]}},\"required\":[\"inner\"],\"additionalProperties\":false,\"$defs\":{\"grow_inner_1\":{\"type\":\"object\",\"properties\":{\"n\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"n\"],\"additionalProperties\":false}}}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order, built
/// whole by its initialiser, every field written: no type has a value nobody wrote
/// (docs/adr/0099, section 4). A value, so a host that reads one holds a copy.
public struct GrowMaybe1Payload: Equatable, Sendable {
    public var inner: GrowInner1Payload?

    public init(inner: GrowInner1Payload?) {
        self.inner = inner
    }
}
