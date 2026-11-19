// SCE-GENERATED — DO NOT EDIT
// source-hash: d3bc03a512d0fa842b2f1c4b18a828c80807d0996aa24cc0dc8422f6f6318938
// source: sce:std/acl/acl_edge.scxml
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

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 40

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:acl_edge:1"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:acl_edge:1\",\"x-sce-source-hash\":\"d3bc03a512d0fa842b2f1c4b18a828c80807d0996aa24cc0dc8422f6f6318938\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"member\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"group\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295}},\"required\":[\"member\",\"group\"],\"additionalProperties\":false}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order, built
/// whole by its initialiser, every field written: no type has a value nobody wrote
/// (docs/adr/0099, section 4). A value, so a host that reads one holds a copy.
public struct AclEdgePayload: Equatable, Sendable {
    public var member: UInt32
    public var group: UInt32

    public init(member: UInt32, group: UInt32) {
        self.member = member
        self.group = group
    }
}
