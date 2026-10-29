// SCE-GENERATED — DO NOT EDIT
// source-hash: b3df842d016676bf8989832cb164de51027fadf6ca9e7b5543c7bac0c9a13d1a
// SCE-MAP: field_version.scxml:28 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: merge.field_version
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum FieldVersion {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "merge.field_version"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct FieldVersionPayload: Equatable, Sendable, SceDefault {
    public var entityId: UInt64 = 0
    public var fieldId: UInt32 = 0
    public var wallTime: Int64 = 0
    public var counter: UInt32 = 0
    public var nodeId: UInt64 = 0
    public var arrival: UInt64 = 0
    public var batchId: UInt64 = 0
    public var origin: UInt8 = 0
    public var revertsBatch: UInt64 = 0
    public var valueWallTime: Int64 = 0
    public var valueCounter: UInt32 = 0
    public var valueNodeId: UInt64 = 0

    public init() {}

    public init(entityId: UInt64, fieldId: UInt32, wallTime: Int64, counter: UInt32, nodeId: UInt64, arrival: UInt64, batchId: UInt64, origin: UInt8, revertsBatch: UInt64, valueWallTime: Int64, valueCounter: UInt32, valueNodeId: UInt64) {
        self.entityId = entityId
        self.fieldId = fieldId
        self.wallTime = wallTime
        self.counter = counter
        self.nodeId = nodeId
        self.arrival = arrival
        self.batchId = batchId
        self.origin = origin
        self.revertsBatch = revertsBatch
        self.valueWallTime = valueWallTime
        self.valueCounter = valueCounter
        self.valueNodeId = valueNodeId
    }
}
