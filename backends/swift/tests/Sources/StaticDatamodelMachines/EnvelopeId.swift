// SCE-GENERATED — DO NOT EDIT
// source-hash: c868fafd129dfb9c6768c3d112e14542a926c118aae9597ed39656faf85e6076
// SCE-MAP: envelope_id.scxml:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: mesh.envelope_id
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum EnvelopeId {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "mesh.envelope_id"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 57
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct EnvelopeIdPayload: Equatable, Sendable, SceDefault {
    public var hi: UInt64 = 0
    public var lo: UInt64 = 0

    public init() {}

    public init(hi: UInt64, lo: UInt64) {
        self.hi = hi
        self.lo = lo
    }
}
