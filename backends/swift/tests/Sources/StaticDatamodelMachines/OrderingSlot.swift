// SCE-GENERATED — DO NOT EDIT
// source-hash: c868fafd129dfb9c6768c3d112e14542a926c118aae9597ed39656faf85e6076
// SCE-MAP: ordering_slot.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: mesh.ordering_slot
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum OrderingSlot {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "mesh.ordering_slot"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 67
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct OrderingSlotPayload: Equatable, Sendable, SceDefault {
    public var seq: UInt64 = 0
    public var arrivedAtMs: Int64 = 0

    public init() {}

    public init(seq: UInt64, arrivedAtMs: Int64) {
        self.seq = seq
        self.arrivedAtMs = arrivedAtMs
    }
}
