// SCE-GENERATED — DO NOT EDIT
// source-hash: 75f53f6deefa1a9e19646e018ad72cacc30c4c7ca7f7f96f21be9f17f58cfdae
// SCE-MAP: closed_enum_form_extras.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: form.closed_enum_extras
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum ClosedEnumFormExtras {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "form.closed_enum_extras"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most elements `slots` holds (the bound its type writes), as this build resolved it.
    public static let slotsMaxCount: UInt32 = 3

    /// The most elements `flags` holds (the bound its type writes), as this build resolved it.
    public static let flagsMaxCount: UInt32 = 2

    /// The most elements `spare` holds (the bound its type writes), as this build resolved it.
    public static let spareMaxCount: UInt32 = 2

    /// The most elements `grid` holds (the bound its type writes), as this build resolved it.
    public static let gridMaxCount: UInt32 = 2

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 129

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:closed_enum_form_extras:1"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:closed_enum_form_extras:1\",\"x-sce-source-hash\":\"75f53f6deefa1a9e19646e018ad72cacc30c4c7ca7f7f96f21be9f17f58cfdae\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"slots\":{\"type\":\"array\",\"items\":{\"anyOf\":[{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535},{\"type\":\"null\"}]},\"maxItems\":3},\"least\":{\"type\":\"string\",\"pattern\":\"^(-([1-9][0-9]{0,17}|[1-8][0-9]{18}|9[0-1][0-9]{17}|92[0-1][0-9]{16}|922[0-2][0-9]{15}|9223[0-2][0-9]{14}|92233[0-6][0-9]{13}|922337[0-1][0-9]{12}|92233720[0-2][0-9]{10}|922337203[0-5][0-9]{9}|9223372036[0-7][0-9]{8}|92233720368[0-4][0-9]{7}|922337203685[0-3][0-9]{6}|9223372036854[0-6][0-9]{5}|92233720368547[0-6][0-9]{4}|922337203685477[0-4][0-9]{3}|9223372036854775[0-7][0-9]{2}|922337203685477580[0-8])|0|[1-9][0-9]{0,17}|[1-8][0-9]{18}|9[0-1][0-9]{17}|92[0-1][0-9]{16}|922[0-2][0-9]{15}|9223[0-2][0-9]{14}|92233[0-6][0-9]{13}|922337[0-1][0-9]{12}|92233720[0-2][0-9]{10}|922337203[0-5][0-9]{9}|9223372036[0-7][0-9]{8}|92233720368[0-4][0-9]{7}|922337203685[0-3][0-9]{6}|9223372036854[0-6][0-9]{5}|92233720368547[0-6][0-9]{4}|922337203685477[0-4][0-9]{3}|9223372036854775[0-7][0-9]{2}|922337203685477580[0-7])$\"},\"flags\":{\"type\":\"array\",\"items\":{\"type\":\"boolean\"},\"minItems\":1,\"maxItems\":2},\"spare\":{\"anyOf\":[{\"type\":\"array\",\"items\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"maxItems\":2},{\"type\":\"null\"}]},\"grid\":{\"type\":\"array\",\"items\":{\"type\":\"array\",\"items\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"maxItems\":2},\"maxItems\":2}},\"required\":[\"slots\",\"least\",\"flags\",\"spare\",\"grid\"],\"additionalProperties\":false}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order, built
/// whole by its initialiser, every field written: no type has a value nobody wrote
/// (docs/adr/0099, section 4). A value, so a host that reads one holds a copy.
public struct ClosedEnumFormExtrasPayload: Equatable, Sendable {
    public var slots: [UInt16?]
    public var least: Int64
    public var flags: [Bool]
    public var spare: [UInt8]?
    public var grid: [[UInt8]]

    public init(slots: [UInt16?], least: Int64, flags: [Bool], spare: [UInt8]?, grid: [[UInt8]]) {
        self.slots = slots
        self.least = least
        self.flags = flags
        self.spare = spare
        self.grid = grid
    }
}
