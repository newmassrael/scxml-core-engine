// SCE-GENERATED — DO NOT EDIT
// source-hash: 1e96d54a1b4f9666059e38585db03a53d8ce664a6f48e0045468eeffc6fb717b
// SCE-MAP: change_entry.scxml:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: sync.change_entry
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum ChangeEntry {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "sync.change_entry"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct ChangeEntryPayload: Equatable, Sendable, SceDefault {
    public var item: UInt64 = 0
    public var op: UInt8 = 0
    public var token: UInt64 = 0

    public init() {}

    public init(item: UInt64, op: UInt8, token: UInt64) {
        self.item = item
        self.op = op
        self.token = token
    }
}
