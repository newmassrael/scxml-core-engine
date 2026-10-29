// SCE-GENERATED — DO NOT EDIT
// source-hash: b3df842d016676bf8989832cb164de51027fadf6ca9e7b5543c7bac0c9a13d1a
// SCE-MAP: sheet_shown.scxml:16 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: merge.sheet_shown
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum SheetShown {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "merge.sheet_shown"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct SheetShownPayload: Equatable, Sendable, SceDefault {
    public var row: UInt64 = 0
    public var col: UInt64 = 0
    public var value: Int64 = 0
    public var fault: UInt8 = 0

    public init() {}

    public init(row: UInt64, col: UInt64, value: Int64, fault: UInt8) {
        self.row = row
        self.col = col
        self.value = value
        self.fault = fault
    }
}
