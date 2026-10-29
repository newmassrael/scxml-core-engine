// SCE-GENERATED — DO NOT EDIT
// source-hash: 1178ba59796ca1f7c46f8569fd4bfda85c2b97db65f51728f3615d990d97dd1c
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
