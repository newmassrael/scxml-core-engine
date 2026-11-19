// SCE-GENERATED — DO NOT EDIT
// source-hash: 84316cd62ded95c431d6e6a91cda76565f65491ed180cf702fb630a2c90862b7
// SCE-MAP: cose_encrypt0_request.scxml:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: host.cose.encrypt0.request
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum CoseEncrypt0Request {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "host.cose.encrypt0.request"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most UTF-8 bytes `key_id` holds (the bound its type writes), as this build resolved it.
    public static let key_idMaxSize: UInt32 = 64

    /// The most bytes `plaintext` holds (the bound its type writes), as this build resolved it.
    public static let plaintextMaxSize: UInt32 = 4352

    /// The most bytes `external_aad` holds (the bound its type writes), as this build resolved it.
    public static let external_aadMaxSize: UInt32 = 256

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 28078

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:cose_encrypt0_request:1"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:cose_encrypt0_request:1\",\"x-sce-source-hash\":\"84316cd62ded95c431d6e6a91cda76565f65491ed180cf702fb630a2c90862b7\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"key_id\":{\"type\":\"string\",\"maxLength\":64,\"x-sce-max-bytes\":64},\"plaintext\":{\"type\":\"string\",\"maxLength\":4352,\"pattern\":\"^[\\\\u0000-\\\\u00ff]*$\"},\"external_aad\":{\"type\":\"string\",\"maxLength\":256,\"pattern\":\"^[\\\\u0000-\\\\u00ff]*$\"}},\"required\":[\"key_id\",\"plaintext\",\"external_aad\"],\"additionalProperties\":false}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order, built
/// whole by its initialiser, every field written: no type has a value nobody wrote
/// (docs/adr/0099, section 4). A value, so a host that reads one holds a copy.
public struct CoseEncrypt0RequestPayload: Equatable, Sendable {
    public var key_id: String
    public var plaintext: [UInt8]
    public var external_aad: [UInt8]

    public init(key_id: String, plaintext: [UInt8], external_aad: [UInt8]) {
        self.key_id = key_id
        self.plaintext = plaintext
        self.external_aad = external_aad
    }
}
