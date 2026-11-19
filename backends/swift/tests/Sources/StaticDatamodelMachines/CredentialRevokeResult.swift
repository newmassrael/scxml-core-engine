// SCE-GENERATED — DO NOT EDIT
// source-hash: 35234879d6c01e7662e45f1ef80491a424ba6e86a5fcba46f1dae0909cdc4cc2
// source: sce:std/host/credential_revoke_result.scxml
// SCE-MAP: credential_revoke_result.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: host.credential.revoke.result
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum CredentialRevokeResult {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "host.credential.revoke.result"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most UTF-8 bytes `error` holds (the bound its type writes), as this build resolved it.
    public static let errorMaxSize: UInt32 = 24

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 177

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:credential_revoke_result:1"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:credential_revoke_result:1\",\"x-sce-source-hash\":\"35234879d6c01e7662e45f1ef80491a424ba6e86a5fcba46f1dae0909cdc4cc2\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"error\":{\"type\":\"string\",\"maxLength\":24,\"x-sce-max-bytes\":24},\"revoked\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295}},\"required\":[\"error\",\"revoked\"],\"additionalProperties\":false}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order, built
/// whole by its initialiser, every field written: no type has a value nobody wrote
/// (docs/adr/0099, section 4). A value, so a host that reads one holds a copy.
public struct CredentialRevokeResultPayload: Equatable, Sendable {
    public var error: String
    public var revoked: UInt32

    public init(error: String, revoked: UInt32) {
        self.error = error
        self.revoked = revoked
    }
}
