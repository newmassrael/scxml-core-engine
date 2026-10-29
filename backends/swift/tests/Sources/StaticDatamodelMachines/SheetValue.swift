// SCE-GENERATED — DO NOT EDIT
// source-hash: b3df842d016676bf8989832cb164de51027fadf6ca9e7b5543c7bac0c9a13d1a
// SCE-MAP: sheet_value.scxml:13 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: merge.sheet_value
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum SheetValue {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "merge.sheet_value"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct SheetValuePayload: Equatable, Sendable, SceDefault {
    public var value: Int64 = 0
    public var fault: UInt8 = 0

    public init() {}

    public init(value: Int64, fault: UInt8) {
        self.value = value
        self.fault = fault
    }
}
