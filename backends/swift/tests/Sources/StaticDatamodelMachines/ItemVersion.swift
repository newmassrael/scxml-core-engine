// SCE-GENERATED — DO NOT EDIT
// source-hash: 9e46d5ba618e2f5a559e7793b0e0ddd1d7ba8428fd3d96128f06266cf4750b56
// SCE-MAP: item_version.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: sync.item_version
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum ItemVersion {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "sync.item_version"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 62
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct ItemVersionPayload: Equatable, Sendable, SceDefault {
    public var item: UInt64 = 0
    public var token: UInt64 = 0

    public init() {}

    public init(item: UInt64, token: UInt64) {
        self.item = item
        self.token = token
    }
}
