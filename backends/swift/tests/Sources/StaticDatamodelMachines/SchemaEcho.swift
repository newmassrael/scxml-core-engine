// SCE-GENERATED — DO NOT EDIT
// source-hash: d947250ac8dbe681be6c82964fb8feadd2bb3cbd6b46adffc9d553a7fc0e7ce6
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

    /// The most UTF-8 bytes `tag` holds (the bound its type writes), as this build resolved it.
    public static let tagMaxSize: UInt32 = 32

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 232
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
