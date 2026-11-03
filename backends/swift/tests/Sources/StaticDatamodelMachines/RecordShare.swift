// SCE-GENERATED — DO NOT EDIT
// source-hash: 26e5bf730b3c2b04bc64b3f3339bd11d68362c63297741a6a051afa4c6cd0d1b
// SCE-MAP: record_share.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: record_list.share
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum RecordShare {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "record_list.share"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 61
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct RecordSharePayload: Equatable, Sendable, SceDefault {
    public var to: UInt64 = 0
    public var minute: UInt64 = 0

    public init() {}

    public init(to: UInt64, minute: UInt64) {
        self.to = to
        self.minute = minute
    }
}
