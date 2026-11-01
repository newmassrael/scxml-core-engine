// SCE-GENERATED — DO NOT EDIT
// source-hash: 24f1c455257ea04f598b006a589c3dea642c1ea2e58b2dcf4b49b8cebf3f207e
// SCE-MAP: todo_room.scxml:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: todo.room
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum TodoRoom {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "todo.room"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct TodoRoomPayload: Equatable, Sendable, SceDefault {
    public var values: UInt32 = 0
    public var history: UInt32 = 0
    public var digest: UInt32 = 0

    public init() {}

    public init(values: UInt32, history: UInt32, digest: UInt32) {
        self.values = values
        self.history = history
        self.digest = digest
    }
}
