// SCE-GENERATED — DO NOT EDIT
// source-hash: 50ba4afa70e08ed0ad002509e6baae9623e17ddc8afb219bbba6da905f187359
// SCE-MAP: event_shapes.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: shapes.sent
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum EventShapes {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "shapes.sent"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most elements `many` holds (the bound its type writes), as this build resolved it.
    public static let manyMaxCount: UInt32 = 3

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 569

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:event_shapes:1"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:event_shapes:1\",\"x-sce-source-hash\":\"50ba4afa70e08ed0ad002509e6baae9623e17ddc8afb219bbba6da905f187359\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"one\":{\"$ref\":\"#/$defs/union_shape\"},\"many\":{\"type\":\"array\",\"items\":{\"$ref\":\"#/$defs/union_shape\"},\"minItems\":1,\"maxItems\":3},\"maybe\":{\"anyOf\":[{\"$ref\":\"#/$defs/union_shape\"},{\"type\":\"null\"}]}},\"required\":[\"one\",\"many\",\"maybe\"],\"additionalProperties\":false,\"$defs\":{\"union_shape\":{\"oneOf\":[{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"dot\"}},\"required\":[\"kind\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"circle\"},\"radius\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"kind\",\"radius\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"disc\"},\"radius\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"kind\",\"radius\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"rect\"},\"width\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535},\"height\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"kind\",\"width\",\"height\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"label\"},\"text\":{\"type\":\"string\",\"maxLength\":8,\"x-sce-max-bytes\":8},\"note\":{\"anyOf\":[{\"type\":\"string\",\"maxLength\":4,\"x-sce-max-bytes\":4},{\"type\":\"null\"}]}},\"required\":[\"kind\",\"text\",\"note\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"path\"},\"points\":{\"type\":\"array\",\"items\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"maxItems\":3}},\"required\":[\"kind\",\"points\"],\"additionalProperties\":false}]}}}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order. A field
/// that holds a union directly has no value to start at (docs/adr/0092, amendment of
/// 2026-11-07: no case is the zero of the others), so the payload has no zero either: it is
/// built whole by its initialiser and is no `SceDefault`. A value, so a host that reads one
/// holds a copy.
public struct EventShapesPayload: Equatable, Sendable {
    public var one: UnionShape
    public var many: [UnionShape] = []
    public var maybe: UnionShape? = nil

    public init(one: UnionShape, many: [UnionShape] = [], maybe: UnionShape? = nil) {
        self.one = one
        self.many = many
        self.maybe = maybe
    }
}
