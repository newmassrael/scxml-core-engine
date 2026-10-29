// SCE-GENERATED — DO NOT EDIT
// source-hash: b67ac92c47dabab09bc66be64b0a4b4a6acfabe17f95412102ed640a717fc04f
// SCE-MAP: schema_echo.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: echo
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum SchemaEcho {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "echo"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct SchemaEchoPayload: Equatable, Sendable, SceDefault {
    public var total: UInt32 = 0
    public var ok: Bool = false
    public var tag: String = ""

    public init() {}

    public init(total: UInt32, ok: Bool, tag: String) {
        self.total = total
        self.ok = ok
        self.tag = tag
    }
}
