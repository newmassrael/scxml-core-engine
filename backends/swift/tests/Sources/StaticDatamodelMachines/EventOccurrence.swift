// SCE-GENERATED — DO NOT EDIT
// source-hash: d947250ac8dbe681be6c82964fb8feadd2bb3cbd6b46adffc9d553a7fc0e7ce6
// SCE-MAP: event_occurrence.scxml:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: calendar.occurrence
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum EventOccurrence {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "calendar.occurrence"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 121
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct EventOccurrencePayload: Equatable, Sendable, SceDefault {
    public var id: UInt64 = 0
    public var start: Int64 = 0
    public var end: Int64 = 0
    public var nth: UInt32 = 0
    public var conflict: UInt8 = 0

    public init() {}

    public init(id: UInt64, start: Int64, end: Int64, nth: UInt32, conflict: UInt8) {
        self.id = id
        self.start = start
        self.end = end
        self.nth = nth
        self.conflict = conflict
    }
}
