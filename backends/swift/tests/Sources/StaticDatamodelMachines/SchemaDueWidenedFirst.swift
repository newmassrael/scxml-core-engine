// SCE-GENERATED — DO NOT EDIT
// source-hash: 1178ba59796ca1f7c46f8569fd4bfda85c2b97db65f51728f3615d990d97dd1c
// SCE-MAP: schema_due_widened_first.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: due.widened
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum SchemaDueWidenedFirst {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "due.widened"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct SchemaDueWidenedFirstPayload: Equatable, Sendable, SceDefault {
    public var id: UInt32 = 0
    public var due: Int64 = 0

    public init() {}

    public init(id: UInt32, due: Int64) {
        self.id = id
        self.due = due
    }
}
