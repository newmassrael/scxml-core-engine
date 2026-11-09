// SCE-GENERATED — DO NOT EDIT
// source-hash: a7faa6958690c5f997097ffa5111459d925f4ceff9942f639c58478c1f89cf17
// SCE-MAP: record_retire.scxml:19 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: record_list.retire
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum RecordRetire {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "record_list.retire"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 59

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:record_retire:1"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:record_retire:1\",\"x-sce-source-hash\":\"a7faa6958690c5f997097ffa5111459d925f4ceff9942f639c58478c1f89cf17\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"id\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"now\":{\"type\":\"string\",\"pattern\":\"^(-([1-9][0-9]{0,17}|[1-8][0-9]{18}|9[0-1][0-9]{17}|92[0-1][0-9]{16}|922[0-2][0-9]{15}|9223[0-2][0-9]{14}|92233[0-6][0-9]{13}|922337[0-1][0-9]{12}|92233720[0-2][0-9]{10}|922337203[0-5][0-9]{9}|9223372036[0-7][0-9]{8}|92233720368[0-4][0-9]{7}|922337203685[0-3][0-9]{6}|9223372036854[0-6][0-9]{5}|92233720368547[0-6][0-9]{4}|922337203685477[0-4][0-9]{3}|9223372036854775[0-7][0-9]{2}|922337203685477580[0-8])|0|[1-9][0-9]{0,17}|[1-8][0-9]{18}|9[0-1][0-9]{17}|92[0-1][0-9]{16}|922[0-2][0-9]{15}|9223[0-2][0-9]{14}|92233[0-6][0-9]{13}|922337[0-1][0-9]{12}|92233720[0-2][0-9]{10}|922337203[0-5][0-9]{9}|9223372036[0-7][0-9]{8}|92233720368[0-4][0-9]{7}|922337203685[0-3][0-9]{6}|9223372036854[0-6][0-9]{5}|92233720368547[0-6][0-9]{4}|922337203685477[0-4][0-9]{3}|9223372036854775[0-7][0-9]{2}|922337203685477580[0-7])$\"},\"more\":{\"type\":\"boolean\"}},\"required\":[\"id\",\"now\",\"more\"],\"additionalProperties\":false}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct RecordRetirePayload: Equatable, Sendable, SceDefault {
    public var id: UInt32 = 0
    public var now: Int64 = 0
    public var more: Bool = false

    public init() {}

    public init(id: UInt32, now: Int64, more: Bool) {
        self.id = id
        self.now = now
        self.more = more
    }
}
