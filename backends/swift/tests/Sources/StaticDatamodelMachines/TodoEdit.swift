// SCE-GENERATED — DO NOT EDIT
// source-hash: ef94c112b4026dfab705629195b17ee265cdc572de6e8fceff328262c2af0613
// SCE-MAP: todo_edit.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: todo.edit
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum TodoEdit {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "todo.edit"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most UTF-8 bytes `text` holds (the bound its type writes), as this build resolved it.
    public static let textMaxSize: UInt32 = 32

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 293
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct TodoEditPayload: Equatable, Sendable, SceDefault {
    public var id: UInt32 = 0
    public var field: UInt16 = 0
    public var value: Int64 = 0
    public var text: String = ""
    public var now: Int64 = 0

    public init() {}

    public init(id: UInt32, field: UInt16, value: Int64, text: String, now: Int64) {
        self.id = id
        self.field = field
        self.value = value
        self.text = text
        self.now = now
    }
}
