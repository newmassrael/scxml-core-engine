// SCE-GENERATED — DO NOT EDIT
// source-hash: 634c022b7eb332297dcb6115177eef8cafdb472ead988f0bdc39838cb7f8d8ae
// SCE-MAP: record_supersede.scxml:19 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: record_list.supersede
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum RecordSupersede {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "record_list.supersede"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 2
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct RecordSupersedePayload: Equatable, Sendable, SceDefault {
    public var id: UInt32 = 0
    public var field: UInt16 = 0
    public var wallTime: Int64 = 0
    public var counter: UInt32 = 0
    public var nodeId: UInt64 = 0
    public var by: UInt64 = 0
    public var parts: UInt32 = 0
    public var atWallTime: Int64 = 0
    public var atCounter: UInt32 = 0

    public init() {}

    public init(id: UInt32, field: UInt16, wallTime: Int64, counter: UInt32, nodeId: UInt64, by: UInt64, parts: UInt32, atWallTime: Int64, atCounter: UInt32) {
        self.id = id
        self.field = field
        self.wallTime = wallTime
        self.counter = counter
        self.nodeId = nodeId
        self.by = by
        self.parts = parts
        self.atWallTime = atWallTime
        self.atCounter = atCounter
    }
}
