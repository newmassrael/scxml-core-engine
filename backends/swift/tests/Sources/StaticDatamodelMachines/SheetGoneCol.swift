// SCE-GENERATED — DO NOT EDIT
// source-hash: ef94c112b4026dfab705629195b17ee265cdc572de6e8fceff328262c2af0613
// SCE-MAP: sheet_gone_col.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: sheet.gone_col
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum SheetGoneCol {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "sheet.gone_col"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 58
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct SheetGoneColPayload: Equatable, Sendable, SceDefault {
    public var key: UInt64 = 0
    public var by: UInt64 = 0

    public init() {}

    public init(key: UInt64, by: UInt64) {
        self.key = key
        self.by = by
    }
}
