// SCE-GENERATED — DO NOT EDIT
// source-hash: e9d14a0e8166097179b777ae40cc80c19551029f15d97332ade0688c2b4c0364
// source: shape_board.scxml
// SCE-MAP: shape_board.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: shape.board
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum ShapeBoard {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "shape.board"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most UTF-8 bytes `title` holds (the bound its type writes), as this build resolved it.
    public static let titleMaxSize: UInt32 = 8

    /// The most UTF-8 bytes `note` holds (the bound its type writes), as this build resolved it.
    public static let noteMaxSize: UInt32 = 8

    /// The most elements `shapes` holds (the bound its type writes), as this build resolved it.
    public static let shapesMaxCount: UInt32 = 3

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 586

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:shape_board:1"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:shape_board:1\",\"x-sce-source-hash\":\"e9d14a0e8166097179b777ae40cc80c19551029f15d97332ade0688c2b4c0364\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"title\":{\"type\":\"string\",\"maxLength\":8,\"x-sce-max-bytes\":8},\"count\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"note\":{\"anyOf\":[{\"type\":\"string\",\"maxLength\":8,\"x-sce-max-bytes\":8},{\"type\":\"null\"}]},\"shapes\":{\"type\":\"array\",\"items\":{\"$ref\":\"#/$defs/union_shape\"},\"maxItems\":3},\"pinned\":{\"anyOf\":[{\"$ref\":\"#/$defs/union_shape\"},{\"type\":\"null\"}]}},\"required\":[\"title\",\"count\",\"note\",\"shapes\",\"pinned\"],\"additionalProperties\":false,\"$defs\":{\"union_shape\":{\"oneOf\":[{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"dot\"}},\"required\":[\"kind\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"circle\"},\"radius\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"kind\",\"radius\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"disc\"},\"radius\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"kind\",\"radius\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"rect\"},\"width\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535},\"height\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"kind\",\"width\",\"height\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"label\"},\"text\":{\"type\":\"string\",\"maxLength\":8,\"x-sce-max-bytes\":8},\"note\":{\"anyOf\":[{\"type\":\"string\",\"maxLength\":4,\"x-sce-max-bytes\":4},{\"type\":\"null\"}]}},\"required\":[\"kind\",\"text\",\"note\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"path\"},\"points\":{\"type\":\"array\",\"items\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"maxItems\":3}},\"required\":[\"kind\",\"points\"],\"additionalProperties\":false}]}}}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order, built
/// whole by its initialiser, every field written: no type has a value nobody wrote
/// (docs/adr/0099, section 4). A value, so a host that reads one holds a copy.
public struct ShapeBoardPayload: Equatable, Sendable {
    public var title: String
    public var count: UInt8
    public var note: String?
    public var shapes: [UnionShape]
    public var pinned: UnionShape?

    public init(title: String, count: UInt8, note: String?, shapes: [UnionShape], pinned: UnionShape?) {
        self.title = title
        self.count = count
        self.note = note
        self.shapes = shapes
        self.pinned = pinned
    }
}
