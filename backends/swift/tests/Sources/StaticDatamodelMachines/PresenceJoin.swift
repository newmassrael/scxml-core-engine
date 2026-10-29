// SCE-GENERATED — DO NOT EDIT
// source-hash: 75c0c87af90d82dec15f516a721dd486d47e1280589aefe06f15efdef5f4f850
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
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct PresenceJoinPayload: Equatable, Sendable, SceDefault {
    public var node: UInt32 = 0
    public var hub: Bool = false
    public var ttl: UInt32 = 0
    public var slack: UInt32 = 0

    public init() {}

    public init(node: UInt32, hub: Bool, ttl: UInt32, slack: UInt32) {
        self.node = node
        self.hub = hub
        self.ttl = ttl
        self.slack = slack
    }
}
