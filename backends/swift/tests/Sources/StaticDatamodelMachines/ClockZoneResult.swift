// SCE-GENERATED — DO NOT EDIT
// source-hash: 3d0f676644ffcd9b4abbc6cf74de18978ed3b2243af3478ce8e8579253b96ea0
// SCE-MAP: clock_zone_result.scxml:13 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: host.clock.zone.result
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum ClockZoneResult {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "host.clock.zone.result"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most UTF-8 bytes `error` holds (the bound its type writes), as this build resolved it.
    public static let errorMaxSize: UInt32 = 24

    /// The most UTF-8 bytes `zone_id` holds (the bound its type writes), as this build resolved it.
    public static let zone_idMaxSize: UInt32 = 64

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 580

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:clock_zone_result:1"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$id\":\"urn:sce:schema:clock_zone_result:1\",\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"additionalProperties\":false,\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"properties\":{\"error\":{\"maxLength\":24,\"type\":\"string\",\"x-sce-max-bytes\":24},\"utc_offset_s\":{\"maximum\":2147483647,\"minimum\":-2147483648,\"type\":\"integer\"},\"zone_id\":{\"maxLength\":64,\"type\":\"string\",\"x-sce-max-bytes\":64}},\"required\":[\"error\",\"utc_offset_s\",\"zone_id\"],\"type\":\"object\",\"x-sce-source-hash\":\"3d0f676644ffcd9b4abbc6cf74de18978ed3b2243af3478ce8e8579253b96ea0\"}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct ClockZoneResultPayload: Equatable, Sendable, SceDefault {
    public var error: String = ""
    public var zone_id: String = ""
    public var utc_offset_s: Int32 = 0

    public init() {}

    public init(error: String, zone_id: String, utc_offset_s: Int32) {
        self.error = error
        self.zone_id = zone_id
        self.utc_offset_s = utc_offset_s
    }
}
