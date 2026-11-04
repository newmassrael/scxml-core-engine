// SCE-GENERATED — DO NOT EDIT
// source-hash: 0c71031c0d27e4d67ce7e4f29c8b0629804b2b22ba0439698c8a940297a0f1bd
// SCE-MAP: civil_date.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: time.civil_date
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum CivilDate {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "time.civil_date"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 42

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:civil_date:1"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$id\":\"urn:sce:schema:civil_date:1\",\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"additionalProperties\":false,\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"properties\":{\"day\":{\"maximum\":255,\"minimum\":0,\"type\":\"integer\"},\"month\":{\"maximum\":255,\"minimum\":0,\"type\":\"integer\"},\"year\":{\"maximum\":2147483647,\"minimum\":-2147483648,\"type\":\"integer\"}},\"required\":[\"day\",\"month\",\"year\"],\"type\":\"object\",\"x-sce-source-hash\":\"0c71031c0d27e4d67ce7e4f29c8b0629804b2b22ba0439698c8a940297a0f1bd\"}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct CivilDatePayload: Equatable, Sendable, SceDefault {
    public var year: Int32 = 0
    public var month: UInt8 = 0
    public var day: UInt8 = 0

    public init() {}

    public init(year: Int32, month: UInt8, day: UInt8) {
        self.year = year
        self.month = month
        self.day = day
    }
}
