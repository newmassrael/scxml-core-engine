// SCE-GENERATED — DO NOT EDIT
// source-hash: 39619f92c6f3b4808e5fe8fb8169e0fd699f0ca38eca6baca0ba1805c3fc2754
// source: grow_outer_2.scxml
// SCE-MAP: grow_outer_2.scxml:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: grow.outer
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum GrowOuter2 {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "grow.outer"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 2

    /// The most elements `inners` holds (the bound its type writes), as this build resolved it.
    public static let innersMaxCount: UInt32 = 2

    /// The most elements `tags` holds (the bound its type writes), as this build resolved it.
    public static let tagsMaxCount: UInt32 = 2

    /// The most UTF-8 bytes `memo` holds (the bound its type writes), as this build resolved it.
    public static let memoMaxSize: UInt32 = 8

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 488

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:grow_outer_2:2"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:grow_outer_2:2\",\"x-sce-source-hash\":\"39619f92c6f3b4808e5fe8fb8169e0fd699f0ca38eca6baca0ba1805c3fc2754\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"id\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"inner\":{\"$ref\":\"#/$defs/grow_inner_2\"},\"inners\":{\"type\":\"array\",\"items\":{\"$ref\":\"#/$defs/grow_inner_2\"},\"maxItems\":2},\"rank\":{\"type\":\"integer\",\"minimum\":-128,\"maximum\":127},\"tags\":{\"type\":\"array\",\"items\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"maxItems\":2},\"leaf\":{\"anyOf\":[{\"$ref\":\"#/$defs/grow_leaf\"},{\"type\":\"null\"}]},\"memo\":{\"anyOf\":[{\"type\":\"string\",\"maxLength\":8,\"x-sce-max-bytes\":8},{\"type\":\"null\"}]}},\"required\":[\"id\",\"inner\",\"inners\",\"rank\",\"tags\",\"leaf\",\"memo\"],\"additionalProperties\":false,\"$defs\":{\"grow_inner_2\":{\"type\":\"object\",\"properties\":{\"n\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535},\"rank\":{\"type\":\"integer\",\"minimum\":-128,\"maximum\":127},\"tags\":{\"type\":\"array\",\"items\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"maxItems\":2},\"leaf\":{\"anyOf\":[{\"$ref\":\"#/$defs/grow_leaf\"},{\"type\":\"null\"}]},\"memo\":{\"anyOf\":[{\"type\":\"string\",\"maxLength\":8,\"x-sce-max-bytes\":8},{\"type\":\"null\"}]}},\"required\":[\"n\",\"rank\",\"tags\",\"leaf\",\"memo\"],\"additionalProperties\":false},\"grow_leaf\":{\"type\":\"object\",\"properties\":{\"v\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255}},\"required\":[\"v\"],\"additionalProperties\":false}}}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order, built
/// whole by its initialiser, every field written: no type has a value nobody wrote
/// (docs/adr/0099, section 4). A value, so a host that reads one holds a copy.
public struct GrowOuter2Payload: Equatable, Sendable {
    public var id: UInt32
    public var inner: GrowInner2Payload
    public var inners: [GrowInner2Payload]
    public var rank: Int8
    public var tags: [UInt8]
    public var leaf: GrowLeafPayload?
    public var memo: String?

    public init(id: UInt32, inner: GrowInner2Payload, inners: [GrowInner2Payload], rank: Int8, tags: [UInt8], leaf: GrowLeafPayload?, memo: String?) {
        self.id = id
        self.inner = inner
        self.inners = inners
        self.rank = rank
        self.tags = tags
        self.leaf = leaf
        self.memo = memo
    }
}
