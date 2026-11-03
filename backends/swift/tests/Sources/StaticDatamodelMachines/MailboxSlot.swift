// SCE-GENERATED — DO NOT EDIT
// source-hash: 9e46d5ba618e2f5a559e7793b0e0ddd1d7ba8428fd3d96128f06266cf4750b56
// SCE-MAP: mailbox_slot.scxml:14 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: sync.mailbox_slot
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum MailboxSlot {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "sync.mailbox_slot"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct MailboxSlotPayload: Equatable, Sendable, SceDefault {
    public var id: Int64 = 0
    public var hi: UInt64 = 0
    public var lo: UInt64 = 0

    public init() {}

    public init(id: Int64, hi: UInt64, lo: UInt64) {
        self.id = id
        self.hi = hi
        self.lo = lo
    }
}
