// SCE-GENERATED — DO NOT EDIT
// source-hash: 1ba579cdc10cbe049a3766817dcc6397d58e8716381e63be0e21ee74788ac719
// SCE-MAP: event_schema_ast_export_min.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: ast.export.min
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum EventSchemaAstExportMin {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "ast.export.min"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct EventSchemaAstExportMinPayload: Equatable, Sendable, SceDefault {
    public var payload_byte: UInt8 = 0

    public init() {}

    public init(payload_byte: UInt8) {
        self.payload_byte = payload_byte
    }
}
