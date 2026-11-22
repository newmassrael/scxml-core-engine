// SCE-GENERATED — DO NOT EDIT
// source-hash: 9849b2de8ad2875f836091ebcb10ca91e9d54a18e96d7509866c583609f2d79b
// source: grow_shaped_2.scxml
// SCE-MAP: grow_shaped_2.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: grow.shaped
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum GrowShaped2 {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "grow.shaped"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 141

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:grow_shaped_2:1"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:grow_shaped_2:1\",\"x-sce-source-hash\":\"9849b2de8ad2875f836091ebcb10ca91e9d54a18e96d7509866c583609f2d79b\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"shape\":{\"$ref\":\"#/$defs/grow_shape_2\"}},\"required\":[\"shape\"],\"additionalProperties\":false,\"$defs\":{\"grow_leaf\":{\"type\":\"object\",\"properties\":{\"v\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255}},\"required\":[\"v\"],\"additionalProperties\":false},\"grow_shape_2\":{\"oneOf\":[{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"single\"},\"n\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535},\"rank\":{\"type\":\"integer\",\"minimum\":-128,\"maximum\":127},\"tags\":{\"type\":\"array\",\"items\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"maxItems\":2},\"leaf\":{\"anyOf\":[{\"$ref\":\"#/$defs/grow_leaf\"},{\"type\":\"null\"}]},\"memo\":{\"anyOf\":[{\"type\":\"string\",\"maxLength\":8,\"x-sce-max-bytes\":8},{\"type\":\"null\"}]}},\"required\":[\"kind\",\"n\",\"rank\",\"tags\",\"leaf\",\"memo\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"none\"}},\"required\":[\"kind\"],\"additionalProperties\":false}]}}}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order, built
/// whole by its initialiser, every field written: no type has a value nobody wrote
/// (docs/adr/0099, section 4). A value, so a host that reads one holds a copy.
public struct GrowShaped2Payload: Equatable, Sendable {
    public var shape: GrowShape2

    public init(shape: GrowShape2) {
        self.shape = shape
    }
}
