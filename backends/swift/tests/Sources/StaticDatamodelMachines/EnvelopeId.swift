// SCE-GENERATED — DO NOT EDIT
// source-hash: 9b4cfcaed7ada488617b52a841abc3e598466613b60c7e1131e1820f7a244b47
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
