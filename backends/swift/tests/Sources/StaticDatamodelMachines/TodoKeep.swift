// SCE-GENERATED — DO NOT EDIT
// source-hash: ef94c112b4026dfab705629195b17ee265cdc572de6e8fceff328262c2af0613
// SCE-MAP: todo_keep.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: todo.keep
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum TodoKeep {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "todo.keep"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 132
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct TodoKeepPayload: Equatable, Sendable, SceDefault {
    public var kind: UInt8 = 0
    public var element: UInt64 = 0
    public var wallTime: Int64 = 0
    public var counter: UInt32 = 0
    public var nodeId: UInt64 = 0

    public init() {}

    public init(kind: UInt8, element: UInt64, wallTime: Int64, counter: UInt32, nodeId: UInt64) {
        self.kind = kind
        self.element = element
        self.wallTime = wallTime
        self.counter = counter
        self.nodeId = nodeId
    }
}
