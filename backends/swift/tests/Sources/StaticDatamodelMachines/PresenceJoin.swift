// SCE-GENERATED — DO NOT EDIT
// source-hash: 65fb42bdd588f958b0fbb29ac261cf039d02606d7afaae0720c045624bd679ba
// SCE-MAP: presence_join.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: presence.join
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum PresenceJoin {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "presence.join"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 67

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:presence_join:1"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:presence_join:1\",\"x-sce-source-hash\":\"65fb42bdd588f958b0fbb29ac261cf039d02606d7afaae0720c045624bd679ba\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"node\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"hub\":{\"type\":\"boolean\"},\"ttl\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"slack\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295}},\"required\":[\"node\",\"hub\",\"ttl\",\"slack\"],\"additionalProperties\":false}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order, built
/// whole by its initialiser, every field written: no type has a value nobody wrote
/// (docs/adr/0099, section 4). A value, so a host that reads one holds a copy.
public struct PresenceJoinPayload: Equatable, Sendable {
    public var node: UInt32
    public var hub: Bool
    public var ttl: UInt32
    public var slack: UInt32

    public init(node: UInt32, hub: Bool, ttl: UInt32, slack: UInt32) {
        self.node = node
        self.hub = hub
        self.ttl = ttl
        self.slack = slack
    }
}
