// SCE-GENERATED — DO NOT EDIT
// source-hash: adbae20e9ca5d62b09870f4a9e5e88119b90dbcf69963958bb021c42c5c2c6a4
// SCE-MAP: schema_wide_relayed.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: wide.relayed
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum SchemaWideRelayed {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "wide.relayed"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct SchemaWideRelayedPayload: Equatable, Sendable, SceDefault {
    public var big: UInt64 = 0
    public var small: Int64 = 0

    public init() {}

    public init(big: UInt64, small: Int64) {
        self.big = big
        self.small = small
    }
}
