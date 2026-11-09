// SCE-GENERATED — DO NOT EDIT
// source-hash: d05ce240c5f3b48b3c83b6bee1ba58feef40d57c310e9a29ef5279604bbaaa42
// SCE-MAP: schema_item_pick.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: item.picked
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum SchemaItemPick {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "item.picked"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most UTF-8 bytes `title` holds (the bound its type writes), as this build resolved it.
    public static let titleMaxSize: UInt32 = 16

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 144

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:schema_item_pick:1"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:schema_item_pick:1\",\"x-sce-source-hash\":\"d05ce240c5f3b48b3c83b6bee1ba58feef40d57c310e9a29ef5279604bbaaa42\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"id\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"title\":{\"type\":\"string\",\"maxLength\":16,\"x-sce-max-bytes\":16},\"count\":{\"type\":\"integer\",\"minimum\":-2147483648,\"maximum\":2147483647}},\"required\":[\"id\",\"title\",\"count\"],\"additionalProperties\":false}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct SchemaItemPickPayload: Equatable, Sendable, SceDefault {
    public var id: UInt32 = 0
    public var title: String = ""
    public var count: Int32 = 0

    public init() {}

    public init(id: UInt32, title: String, count: Int32) {
        self.id = id
        self.title = title
        self.count = count
    }
}
