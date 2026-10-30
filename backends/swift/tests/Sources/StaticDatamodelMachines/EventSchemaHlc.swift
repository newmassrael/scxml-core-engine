// SCE-GENERATED — DO NOT EDIT
// source-hash: f6c860c2693fcb3366bc871ff801c9b0ee9e491a1759830f8d83238267cc3b45
// SCE-MAP: event_schema_hlc.scxml:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: hlc.stamp
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum EventSchemaHlc {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "hlc.stamp"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct EventSchemaHlcPayload: Equatable, Sendable, SceDefault {
    public var wallTime: Int64 = 0
    public var counter: UInt32 = 0
    public var nodeId: UInt64 = 0

    public init() {}

    public init(wallTime: Int64, counter: UInt32, nodeId: UInt64) {
        self.wallTime = wallTime
        self.counter = counter
        self.nodeId = nodeId
    }
}
