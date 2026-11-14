// SCE-GENERATED — DO NOT EDIT
// source-hash: c59281ae7be4758cbffb01ac828a80a4489cab0ae77d7577e991e823091b27bc
// SCE-MAP: acl_privilege.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: acl.privilege
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum AclPrivilege {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "acl.privilege"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 37

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:acl_privilege:1"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:acl_privilege:1\",\"x-sce-source-hash\":\"c59281ae7be4758cbffb01ac828a80a4489cab0ae77d7577e991e823091b27bc\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"id\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"parent\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295}},\"required\":[\"id\",\"parent\"],\"additionalProperties\":false}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order, built
/// whole by its initialiser, every field written: no type has a value nobody wrote
/// (docs/adr/0099, section 4). A value, so a host that reads one holds a copy.
public struct AclPrivilegePayload: Equatable, Sendable {
    public var id: UInt32
    public var parent: UInt32

    public init(id: UInt32, parent: UInt32) {
        self.id = id
        self.parent = parent
    }
}
