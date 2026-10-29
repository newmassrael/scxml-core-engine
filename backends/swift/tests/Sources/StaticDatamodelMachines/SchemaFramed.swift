// SCE-GENERATED — DO NOT EDIT
// source-hash: 75c0c87af90d82dec15f516a721dd486d47e1280589aefe06f15efdef5f4f850
// SCE-MAP: schema_framed.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: framed.taken
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum SchemaFramed {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "framed.taken"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most bytes `frame` holds (`sce:max-size`), as this build resolved it.
    public static let frameMaxSize: UInt32 = 8
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct SchemaFramedPayload: Equatable, Sendable, SceDefault {
    public var sensor: UInt8 = 0
    public var frame: [UInt8] = []

    public init() {}

    public init(sensor: UInt8, frame: [UInt8]) {
        self.sensor = sensor
        self.frame = frame
    }
}
