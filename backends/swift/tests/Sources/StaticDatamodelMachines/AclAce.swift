// SCE-GENERATED — DO NOT EDIT
// source-hash: c59281ae7be4758cbffb01ac828a80a4489cab0ae77d7577e991e823091b27bc
// SCE-MAP: acl_ace.scxml:14 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: acl.ace
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum AclAce {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "acl.ace"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 47
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct AclAcePayload: Equatable, Sendable, SceDefault {
    public var principal: UInt32 = 0
    public var privilege: UInt32 = 0

    public init() {}

    public init(principal: UInt32, privilege: UInt32) {
        self.principal = principal
        self.privilege = privilege
    }
}
