// SCE-GENERATED — DO NOT EDIT
// source-hash: cbae23ec3e271a5d3d031cf17aaa399ae7d046b79eff29ef521b985e000db27e
// source: grow_holding_1.scxml
// SCE-MAP: grow_holding_1.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: grow.holding
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum GrowHolding1 {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "grow.holding"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 82

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:grow_holding_1:1"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:grow_holding_1:1\",\"x-sce-source-hash\":\"cbae23ec3e271a5d3d031cf17aaa399ae7d046b79eff29ef521b985e000db27e\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"outer\":{\"anyOf\":[{\"$ref\":\"#/$defs/grow_outer_1\"},{\"type\":\"null\"}]}},\"required\":[\"outer\"],\"additionalProperties\":false,\"$defs\":{\"grow_inner_1\":{\"type\":\"object\",\"properties\":{\"n\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"n\"],\"additionalProperties\":false},\"grow_outer_1\":{\"type\":\"object\",\"properties\":{\"id\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"inner\":{\"$ref\":\"#/$defs/grow_inner_1\"},\"inners\":{\"type\":\"array\",\"items\":{\"$ref\":\"#/$defs/grow_inner_1\"},\"maxItems\":2}},\"required\":[\"id\",\"inner\",\"inners\"],\"additionalProperties\":false}}}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order, built
/// whole by its initialiser, every field written: no type has a value nobody wrote
/// (docs/adr/0099, section 4). A value, so a host that reads one holds a copy.
public struct GrowHolding1Payload: Equatable, Sendable {
    public var outer: GrowOuter1Payload?

    public init(outer: GrowOuter1Payload?) {
        self.outer = outer
    }
}
