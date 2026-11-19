// SCE-GENERATED — DO NOT EDIT
// source-hash: 02ff4e84ad8d63f0367eba4adea80b75a0ee4bce7afe070400b914a60573428b
// source: sce:std/host/credential_issue_request.scxml
// SCE-MAP: credential_issue_request.scxml:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: host.credential.issue.request
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum CredentialIssueRequest {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "host.credential.issue.request"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most UTF-8 bytes `name` holds (the bound its type writes), as this build resolved it.
    public static let nameMaxSize: UInt32 = 256

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 1566

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:credential_issue_request:1"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:credential_issue_request:1\",\"x-sce-source-hash\":\"02ff4e84ad8d63f0367eba4adea80b75a0ee4bce7afe070400b914a60573428b\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"name\":{\"type\":\"string\",\"maxLength\":256,\"x-sce-max-bytes\":256},\"ttl_s\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295}},\"required\":[\"name\",\"ttl_s\"],\"additionalProperties\":false}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order, built
/// whole by its initialiser, every field written: no type has a value nobody wrote
/// (docs/adr/0099, section 4). A value, so a host that reads one holds a copy.
public struct CredentialIssueRequestPayload: Equatable, Sendable {
    public var name: String
    public var ttl_s: UInt32

    public init(name: String, ttl_s: UInt32) {
        self.name = name
        self.ttl_s = ttl_s
    }
}
