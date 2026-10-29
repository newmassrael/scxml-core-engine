// SCE-GENERATED — DO NOT EDIT
// source-hash: 1178ba59796ca1f7c46f8569fd4bfda85c2b97db65f51728f3615d990d97dd1c
// SCE-MAP: presence_seen.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: presence.seen
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum PresenceSeen {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "presence.seen"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct PresenceSeenPayload: Equatable, Sendable, SceDefault {
    public var client: UInt32 = 0
    public var last: Int64 = 0
    public var gen: UInt32 = 0

    public init() {}

    public init(client: UInt32, last: Int64, gen: UInt32) {
        self.client = client
        self.last = last
        self.gen = gen
    }
}
