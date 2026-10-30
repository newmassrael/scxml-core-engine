// SCE-GENERATED — DO NOT EDIT
// source-hash: 3e850e348999a821018727a6eab60f939f8f51a6e8133ca476ace471d64c40a6
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
