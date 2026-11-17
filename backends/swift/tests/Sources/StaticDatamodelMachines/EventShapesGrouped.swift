// SCE-GENERATED — DO NOT EDIT
// source-hash: 2073d8a55462e1ce61d8a77293fdd2c141aceda8ad173303c5b84813ca22eeee
// SCE-MAP: event_shapes_grouped.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: shapes.grouped
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum EventShapesGrouped {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "shapes.grouped"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most elements `groups` holds (the bound its type writes), as this build resolved it.
    public static let groupsMaxCount: UInt32 = 2

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 713

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:event_shapes_grouped:1"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:event_shapes_grouped:1\",\"x-sce-source-hash\":\"2073d8a55462e1ce61d8a77293fdd2c141aceda8ad173303c5b84813ca22eeee\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"groups\":{\"type\":\"array\",\"items\":{\"$ref\":\"#/$defs/shape_group\"},\"maxItems\":2},\"at\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"inner\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255}},\"required\":[\"groups\",\"at\",\"inner\"],\"additionalProperties\":false,\"$defs\":{\"shape_group\":{\"type\":\"object\",\"properties\":{\"shapes\":{\"type\":\"array\",\"items\":{\"$ref\":\"#/$defs/union_shape\"},\"maxItems\":3}},\"required\":[\"shapes\"],\"additionalProperties\":false},\"union_shape\":{\"oneOf\":[{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"dot\"}},\"required\":[\"kind\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"circle\"},\"radius\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"kind\",\"radius\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"disc\"},\"radius\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"kind\",\"radius\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"rect\"},\"width\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535},\"height\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"kind\",\"width\",\"height\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"label\"},\"text\":{\"type\":\"string\",\"maxLength\":8,\"x-sce-max-bytes\":8},\"note\":{\"anyOf\":[{\"type\":\"string\",\"maxLength\":4,\"x-sce-max-bytes\":4},{\"type\":\"null\"}]}},\"required\":[\"kind\",\"text\",\"note\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"path\"},\"points\":{\"type\":\"array\",\"items\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"maxItems\":3}},\"required\":[\"kind\",\"points\"],\"additionalProperties\":false}]}}}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order, built
/// whole by its initialiser, every field written: no type has a value nobody wrote
/// (docs/adr/0099, section 4). A value, so a host that reads one holds a copy.
public struct EventShapesGroupedPayload: Equatable, Sendable {
    public var groups: [ShapeGroupPayload]
    public var at: UInt8
    public var inner: UInt8

    public init(groups: [ShapeGroupPayload], at: UInt8, inner: UInt8) {
        self.groups = groups
        self.at = at
        self.inner = inner
    }
}
