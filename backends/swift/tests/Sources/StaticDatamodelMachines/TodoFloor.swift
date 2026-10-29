// SCE-GENERATED — DO NOT EDIT
// source-hash: adbae20e9ca5d62b09870f4a9e5e88119b90dbcf69963958bb021c42c5c2c6a4
// SCE-MAP: todo_floor.scxml:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: todo.floor
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum TodoFloor {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "todo.floor"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct TodoFloorPayload: Equatable, Sendable, SceDefault {
    public var before: Int64 = 0
    public var hash: UInt32 = 0

    public init() {}

    public init(before: Int64, hash: UInt32) {
        self.before = before
        self.hash = hash
    }
}
