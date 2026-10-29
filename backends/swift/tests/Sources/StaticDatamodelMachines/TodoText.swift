// SCE-GENERATED — DO NOT EDIT
// source-hash: 75c0c87af90d82dec15f516a721dd486d47e1280589aefe06f15efdef5f4f850
// SCE-MAP: todo_text.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: todo.text
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum TodoText {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "todo.text"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most UTF-8 bytes `text` holds (`sce:max-size`), as this build resolved it.
    public static let textMaxSize: UInt32 = 32
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct TodoTextPayload: Equatable, Sendable, SceDefault {
    public var id: UInt64 = 0
    public var field: UInt64 = 0
    public var text: String = ""

    public init() {}

    public init(id: UInt64, field: UInt64, text: String) {
        self.id = id
        self.field = field
        self.text = text
    }
}
