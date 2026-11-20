// SCE-GENERATED — DO NOT EDIT
// source-hash: 0ad70b1df3fdd96ec1d9ed7bb9255fafcfa055068b5d7a725c84cf01e0b6e595
// source: schema_audit_sign_in.scxml
// SCE-MAP: schema_audit_sign_in.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: audit.sign_in
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum SchemaAuditSignIn {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "audit.sign_in"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most UTF-8 bytes `account` holds (the bound its type writes), as this build resolved it.
    public static let accountMaxSize: UInt32 = 64

    /// The most UTF-8 bytes `password` holds (the bound its type writes), as this build resolved it.
    public static let passwordMaxSize: UInt32 = 32

    /// The most UTF-8 bytes `result` holds (the bound its type writes), as this build resolved it.
    public static let resultMaxSize: UInt32 = 16

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 733

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:schema_audit_sign_in:1"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:schema_audit_sign_in:1\",\"x-sce-source-hash\":\"0ad70b1df3fdd96ec1d9ed7bb9255fafcfa055068b5d7a725c84cf01e0b6e595\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"account\":{\"type\":\"string\",\"maxLength\":64,\"x-sce-max-bytes\":64},\"password\":{\"type\":\"string\",\"maxLength\":32,\"x-sce-max-bytes\":32},\"result\":{\"type\":\"string\",\"maxLength\":16,\"x-sce-max-bytes\":16},\"attempt\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295}},\"required\":[\"account\",\"password\",\"result\",\"attempt\"],\"additionalProperties\":false}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order, built
/// whole by its initialiser, every field written: no type has a value nobody wrote
/// (docs/adr/0099, section 4). A value, so a host that reads one holds a copy.
public struct SchemaAuditSignInPayload: Equatable, Sendable {
    public var account: String
    public var password: String
    public var result: String
    public var attempt: UInt32

    public init(account: String, password: String, result: String, attempt: UInt32) {
        self.account = account
        self.password = password
        self.result = result
        self.attempt = attempt
    }
}
