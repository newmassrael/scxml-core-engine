// SCE-GENERATED — DO NOT EDIT
// source-hash: c4657e911a812ec796ce0b3c5be3eeef2b66326118a66c11d56342e555c6a85c
// SCE-MAP: schema_day.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: day.picked
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum SchemaDay {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "day.picked"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 43
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct SchemaDayPayload: Equatable, Sendable, SceDefault {
    public var year: UInt16 = 0
    public var month: UInt8 = 0
    public var dayOfMonth: UInt8 = 0

    public init() {}

    public init(year: UInt16, month: UInt8, dayOfMonth: UInt8) {
        self.year = year
        self.month = month
        self.dayOfMonth = dayOfMonth
    }
}
