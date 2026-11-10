// SCE-GENERATED — DO NOT EDIT
// source-hash: 43faded87b84ec06f387d0e3d6ccb626e2fa1802d2395c65b70588a4c3c394af
// SCE-MAP: event_counted.scxml:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: counted.sent
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum EventCounted {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "counted.sent"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most elements `kept` holds (the bound its type writes), as this build resolved it.
    public static let keptMaxCount: UInt32 = 2

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 29

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:event_counted:1"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:event_counted:1\",\"x-sce-source-hash\":\"43faded87b84ec06f387d0e3d6ccb626e2fa1802d2395c65b70588a4c3c394af\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"kept\":{\"type\":\"array\",\"items\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"minItems\":1,\"maxItems\":2},\"hops\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255}},\"required\":[\"kept\",\"hops\"],\"additionalProperties\":false}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct EventCountedPayload: Equatable, Sendable, SceDefault {
    public var kept: [UInt8] = []
    public var hops: UInt8 = 0

    public init() {}

    public init(kept: [UInt8], hops: UInt8) {
        self.kept = kept
        self.hops = hops
    }
}
