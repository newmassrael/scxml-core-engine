// SCE-GENERATED — DO NOT EDIT
// source-hash: 75c0c87af90d82dec15f516a721dd486d47e1280589aefe06f15efdef5f4f850
// SCE-MAP: todo_join.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: todo.join
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum TodoJoin {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "todo.join"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct TodoJoinPayload: Equatable, Sendable, SceDefault {
    public var node: UInt64 = 0
    public var hub: Bool = false

    public init() {}

    public init(node: UInt64, hub: Bool) {
        self.node = node
        self.hub = hub
    }
}
