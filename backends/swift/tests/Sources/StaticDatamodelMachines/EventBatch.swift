// SCE-GENERATED — DO NOT EDIT
// source-hash: 65fb42bdd588f958b0fbb29ac261cf039d02606d7afaae0720c045624bd679ba
// SCE-MAP: event_batch.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: batch.sent
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum EventBatch {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "batch.sent"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 2

    /// The most elements `items` holds (the bound its type writes, named `batch_items`), as this build resolved it.
    public static let itemsMaxCount: UInt32 = 3

    /// The most elements `grid` holds (the bound its type writes), as this build resolved it.
    public static let gridMaxCount: UInt32 = 2

    /// The most elements `levels` holds (the bound its type writes), as this build resolved it.
    public static let levelsMaxCount: UInt32 = 2

    /// The most elements `maybe` holds (the bound its type writes), as this build resolved it.
    public static let maybeMaxCount: UInt32 = 2

    /// The most elements `spare` holds (the bound its type writes), as this build resolved it.
    public static let spareMaxCount: UInt32 = 2

    /// The most UTF-8 bytes `note` holds (the bound its type writes), as this build resolved it.
    public static let noteMaxSize: UInt32 = 4

    /// The most elements `late` holds (the bound its type writes), as this build resolved it.
    public static let lateMaxCount: UInt32 = 2

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 1376

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:event_batch:2"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:event_batch:2\",\"x-sce-source-hash\":\"65fb42bdd588f958b0fbb29ac261cf039d02606d7afaae0720c045624bd679ba\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"items\":{\"type\":\"array\",\"items\":{\"$ref\":\"#/$defs/event_batch_item\"},\"maxItems\":3},\"grid\":{\"type\":\"array\",\"items\":{\"type\":\"array\",\"items\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"maxItems\":2},\"maxItems\":2},\"levels\":{\"type\":\"array\",\"items\":{\"enum\":[\"low\",\"high\",\"critical\"]},\"maxItems\":2},\"maybe\":{\"type\":\"array\",\"items\":{\"anyOf\":[{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535},{\"type\":\"null\"}]},\"maxItems\":2},\"spare\":{\"anyOf\":[{\"type\":\"array\",\"items\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"maxItems\":2},{\"type\":\"null\"}]},\"note\":{\"type\":\"string\",\"maxLength\":4,\"x-sce-max-bytes\":4},\"late\":{\"type\":\"array\",\"items\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"maxItems\":2}},\"required\":[\"items\",\"grid\",\"levels\",\"maybe\",\"spare\",\"note\",\"late\"],\"additionalProperties\":false,\"$defs\":{\"event_batch_item\":{\"type\":\"object\",\"properties\":{\"id\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"amount\":{\"type\":\"string\",\"pattern\":\"^(-([1-9][0-9]{0,17}|[1-8][0-9]{18}|9[0-1][0-9]{17}|92[0-1][0-9]{16}|922[0-2][0-9]{15}|9223[0-2][0-9]{14}|92233[0-6][0-9]{13}|922337[0-1][0-9]{12}|92233720[0-2][0-9]{10}|922337203[0-5][0-9]{9}|9223372036[0-7][0-9]{8}|92233720368[0-4][0-9]{7}|922337203685[0-3][0-9]{6}|9223372036854[0-6][0-9]{5}|92233720368547[0-6][0-9]{4}|922337203685477[0-4][0-9]{3}|9223372036854775[0-7][0-9]{2}|922337203685477580[0-8])|0|[1-9][0-9]{0,17}|[1-8][0-9]{18}|9[0-1][0-9]{17}|92[0-1][0-9]{16}|922[0-2][0-9]{15}|9223[0-2][0-9]{14}|92233[0-6][0-9]{13}|922337[0-1][0-9]{12}|92233720[0-2][0-9]{10}|922337203[0-5][0-9]{9}|9223372036[0-7][0-9]{8}|92233720368[0-4][0-9]{7}|922337203685[0-3][0-9]{6}|9223372036854[0-6][0-9]{5}|92233720368547[0-6][0-9]{4}|922337203685477[0-4][0-9]{3}|9223372036854775[0-7][0-9]{2}|922337203685477580[0-7])$\"},\"label\":{\"type\":\"string\",\"maxLength\":8,\"x-sce-max-bytes\":8},\"level\":{\"enum\":[\"low\",\"high\",\"critical\"]},\"due\":{\"anyOf\":[{\"type\":\"string\",\"pattern\":\"^(-([1-9][0-9]{0,17}|[1-8][0-9]{18}|9[0-1][0-9]{17}|92[0-1][0-9]{16}|922[0-2][0-9]{15}|9223[0-2][0-9]{14}|92233[0-6][0-9]{13}|922337[0-1][0-9]{12}|92233720[0-2][0-9]{10}|922337203[0-5][0-9]{9}|9223372036[0-7][0-9]{8}|92233720368[0-4][0-9]{7}|922337203685[0-3][0-9]{6}|9223372036854[0-6][0-9]{5}|92233720368547[0-6][0-9]{4}|922337203685477[0-4][0-9]{3}|9223372036854775[0-7][0-9]{2}|922337203685477580[0-8])|0|[1-9][0-9]{0,17}|[1-8][0-9]{18}|9[0-1][0-9]{17}|92[0-1][0-9]{16}|922[0-2][0-9]{15}|9223[0-2][0-9]{14}|92233[0-6][0-9]{13}|922337[0-1][0-9]{12}|92233720[0-2][0-9]{10}|922337203[0-5][0-9]{9}|9223372036[0-7][0-9]{8}|92233720368[0-4][0-9]{7}|922337203685[0-3][0-9]{6}|9223372036854[0-6][0-9]{5}|92233720368547[0-6][0-9]{4}|922337203685477[0-4][0-9]{3}|9223372036854775[0-7][0-9]{2}|922337203685477580[0-7])$\"},{\"type\":\"null\"}]},\"blob\":{\"type\":\"string\",\"maxLength\":4,\"pattern\":\"^[\\\\u0000-\\\\u00ff]*$\"},\"tags\":{\"type\":\"array\",\"items\":{\"type\":\"string\",\"maxLength\":8,\"x-sce-max-bytes\":8},\"maxItems\":4}},\"required\":[\"id\",\"amount\",\"label\",\"level\",\"due\",\"blob\",\"tags\"],\"additionalProperties\":false}}}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order, built
/// whole by its initialiser, every field written: no type has a value nobody wrote
/// (docs/adr/0099, section 4). A value, so a host that reads one holds a copy.
public struct EventBatchPayload: Equatable, Sendable {
    public var items: [EventBatchItemPayload]
    public var grid: [[UInt8]]
    public var levels: [EnumSignalLevel]
    public var maybe: [UInt16?]
    public var spare: [UInt8]?
    public var note: String
    public var late: [UInt8]

    public init(items: [EventBatchItemPayload], grid: [[UInt8]], levels: [EnumSignalLevel], maybe: [UInt16?], spare: [UInt8]?, note: String, late: [UInt8]) {
        self.items = items
        self.grid = grid
        self.levels = levels
        self.maybe = maybe
        self.spare = spare
        self.note = note
        self.late = late
    }
}
