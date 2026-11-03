// SCE-GENERATED — DO NOT EDIT
// source-hash: d947250ac8dbe681be6c82964fb8feadd2bb3cbd6b46adffc9d553a7fc0e7ce6
// SCE-MAP: todo_node.scxml:13 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: todo.node
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum TodoNode {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "todo.node"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 137
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct TodoNodePayload: Equatable, Sendable, SceDefault {
    public var sender: UInt64 = 0
    public var to: UInt64 = 0
    public var depth: UInt8 = 0
    public var prefix: UInt64 = 0
    public var hash: UInt32 = 0
    public var reply: Bool = false

    public init() {}

    public init(sender: UInt64, to: UInt64, depth: UInt8, prefix: UInt64, hash: UInt32, reply: Bool) {
        self.sender = sender
        self.to = to
        self.depth = depth
        self.prefix = prefix
        self.hash = hash
        self.reply = reply
    }
}
