// SCE-GENERATED — DO NOT EDIT
// source-hash: 1178ba59796ca1f7c46f8569fd4bfda85c2b97db65f51728f3615d990d97dd1c
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
