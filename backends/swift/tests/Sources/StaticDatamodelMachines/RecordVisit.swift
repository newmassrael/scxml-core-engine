// SCE-GENERATED — DO NOT EDIT
// source-hash: 634c022b7eb332297dcb6115177eef8cafdb472ead988f0bdc39838cb7f8d8ae
// SCE-MAP: record_visit.scxml:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: record_list.visit
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum RecordVisit {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "record_list.visit"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct RecordVisitPayload: Equatable, Sendable, SceDefault {
    public var to: UInt64 = 0
    public var depth: UInt8 = 0
    public var prefix: UInt64 = 0
    public var reply: Bool = false

    public init() {}

    public init(to: UInt64, depth: UInt8, prefix: UInt64, reply: Bool) {
        self.to = to
        self.depth = depth
        self.prefix = prefix
        self.reply = reply
    }
}
