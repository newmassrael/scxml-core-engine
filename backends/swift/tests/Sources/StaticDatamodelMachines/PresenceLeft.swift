// SCE-GENERATED — DO NOT EDIT
// source-hash: c4657e911a812ec796ce0b3c5be3eeef2b66326118a66c11d56342e555c6a85c
// SCE-MAP: presence_left.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: presence.left
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum PresenceLeft {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "presence.left"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 51
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct PresenceLeftPayload: Equatable, Sendable, SceDefault {
    public var client: UInt32 = 0
    public var last: Int64 = 0

    public init() {}

    public init(client: UInt32, last: Int64) {
        self.client = client
        self.last = last
    }
}
