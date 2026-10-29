// SCE-GENERATED — DO NOT EDIT
// source-hash: 634c022b7eb332297dcb6115177eef8cafdb472ead988f0bdc39838cb7f8d8ae
// SCE-MAP: record_retire.scxml:19 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: record_list.retire
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum RecordRetire {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "record_list.retire"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct RecordRetirePayload: Equatable, Sendable, SceDefault {
    public var id: UInt32 = 0
    public var now: Int64 = 0
    public var more: Bool = false

    public init() {}

    public init(id: UInt32, now: Int64, more: Bool) {
        self.id = id
        self.now = now
        self.more = more
    }
}
