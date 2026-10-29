// SCE-GENERATED — DO NOT EDIT
// source-hash: 1178ba59796ca1f7c46f8569fd4bfda85c2b97db65f51728f3615d990d97dd1c
// SCE-MAP: schema_revised.scxml:15 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: reading.revised
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum SchemaRevised {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "reading.revised"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 3
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct SchemaRevisedPayload: Equatable, Sendable, SceDefault {
    public var level: UInt8 = 0
    public var layout: EnumViewMode = EnumViewMode.allCases[0]
    public var urgent: Bool = false

    public init() {}

    public init(level: UInt8, layout: EnumViewMode, urgent: Bool) {
        self.level = level
        self.layout = layout
        self.urgent = urgent
    }
}
