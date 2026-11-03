// SCE-GENERATED — DO NOT EDIT
// source-hash: fb4f42a34e7ac40a28d8e5daee5d633c3c45dddac513816edf2bb83fdc85f91c
// SCE-MAP: mv_value.scxml:27 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: merge.mv_value
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum MvValue {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "merge.mv_value"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most UTF-8 bytes `text` holds (`sce:max-size`, the bound `sce:limit="merge_text"` names), as this build resolved it.
    public static let textMaxSize: UInt32 = 96
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct MvValuePayload: Equatable, Sendable, SceDefault {
    public var id: UInt64 = 0
    public var field: UInt64 = 0
    public var value: Int64 = 0
    public var text: String = ""
    public var wallTime: Int64 = 0
    public var counter: UInt32 = 0
    public var nodeId: UInt64 = 0

    public init() {}

    public init(id: UInt64, field: UInt64, value: Int64, text: String, wallTime: Int64, counter: UInt32, nodeId: UInt64) {
        self.id = id
        self.field = field
        self.value = value
        self.text = text
        self.wallTime = wallTime
        self.counter = counter
        self.nodeId = nodeId
    }
}
