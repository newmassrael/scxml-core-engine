// SCE-GENERATED — DO NOT EDIT
// source-hash: 634c022b7eb332297dcb6115177eef8cafdb472ead988f0bdc39838cb7f8d8ae
// SCE-MAP: record_keep.scxml:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: record_list.keep
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum RecordKeep {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "record_list.keep"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct RecordKeepPayload: Equatable, Sendable, SceDefault {
    public var kind: UInt8 = 0
    public var element: UInt64 = 0
    public var wallTime: Int64 = 0
    public var counter: UInt32 = 0
    public var nodeId: UInt64 = 0

    public init() {}

    public init(kind: UInt8, element: UInt64, wallTime: Int64, counter: UInt32, nodeId: UInt64) {
        self.kind = kind
        self.element = element
        self.wallTime = wallTime
        self.counter = counter
        self.nodeId = nodeId
    }
}
