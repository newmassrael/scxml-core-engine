// SCE-GENERATED — DO NOT EDIT
// source-hash: 26e5bf730b3c2b04bc64b3f3339bd11d68362c63297741a6a051afa4c6cd0d1b
// SCE-MAP: record_node.scxml:16 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: record_list.node
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum RecordNode {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "record_list.node"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct RecordNodePayload: Equatable, Sendable, SceDefault {
    public var sender: UInt64 = 0
    public var to: UInt64 = 0
    public var depth: UInt8 = 0
    public var prefix: UInt64 = 0
    public var hash: UInt32 = 0
    public var reply: Bool = false

    public init() {}

    public init(sender: UInt64, to: UInt64, depth: UInt8, prefix: UInt64, hash: UInt32, reply: Bool) {
        self.sender = sender
        self.to = to
        self.depth = depth
        self.prefix = prefix
        self.hash = hash
        self.reply = reply
    }
}
