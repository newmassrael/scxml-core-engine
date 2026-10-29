// SCE-GENERATED — DO NOT EDIT
// source-hash: cabd60d6426d729773396692f3fde7a45257e44f0e01ae8a7f61990f6a4ec86b
// SCE-MAP: acl_edge.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: acl.edge
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum AclEdge {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "acl.edge"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct AclEdgePayload: Equatable, Sendable, SceDefault {
    public var member: UInt32 = 0
    public var group: UInt32 = 0

    public init() {}

    public init(member: UInt32, group: UInt32) {
        self.member = member
        self.group = group
    }
}
