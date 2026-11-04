// SCE-GENERATED — DO NOT EDIT
// source-hash: c4657e911a812ec796ce0b3c5be3eeef2b66326118a66c11d56342e555c6a85c
// SCE-MAP: schema_due.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: due.set
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum SchemaDue {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "due.set"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most UTF-8 bytes `title` holds (the bound its type writes), as this build resolved it.
    public static let titleMaxSize: UInt32 = 16

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 176
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct SchemaDuePayload: Equatable, Sendable, SceDefault {
    public var id: UInt32 = 0
    public var due: Int64? = nil
    public var title: String? = nil
    public var status: EnumItemStatus? = nil

    public init() {}

    public init(id: UInt32, due: Int64?, title: String?, status: EnumItemStatus?) {
        self.id = id
        self.due = due
        self.title = title
        self.status = status
    }
}
