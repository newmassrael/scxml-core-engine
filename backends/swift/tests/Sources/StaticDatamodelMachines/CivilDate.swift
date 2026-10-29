// SCE-GENERATED — DO NOT EDIT
// source-hash: 68d77b5e0c96e361239e1512887ba2ef6d35bfab0188bf0e2afbd53ca387f3bc
// SCE-MAP: civil_date.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: time.civil_date
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum CivilDate {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "time.civil_date"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct CivilDatePayload: Equatable, Sendable, SceDefault {
    public var year: Int32 = 0
    public var month: UInt8 = 0
    public var day: UInt8 = 0

    public init() {}

    public init(year: Int32, month: UInt8, day: UInt8) {
        self.year = year
        self.month = month
        self.day = day
    }
}
