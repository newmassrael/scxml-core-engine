// SCE-GENERATED — DO NOT EDIT
// source-hash: 84316cd62ded95c431d6e6a91cda76565f65491ed180cf702fb630a2c90862b7
// SCE-MAP: key_derive_request.scxml:13 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: host.key.derive.request
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum KeyDeriveRequest {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "host.key.derive.request"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most bytes `secret` holds (the bound its type writes), as this build resolved it.
    public static let secretMaxSize: UInt32 = 256

    /// The most UTF-8 bytes `input_key_id` holds (the bound its type writes), as this build resolved it.
    public static let input_key_idMaxSize: UInt32 = 64

    /// The most bytes `salt` holds (the bound its type writes), as this build resolved it.
    public static let saltMaxSize: UInt32 = 64

    /// The most bytes `info` holds (the bound its type writes), as this build resolved it.
    public static let infoMaxSize: UInt32 = 256

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 3891

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:key_derive_request:1"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:key_derive_request:1\",\"x-sce-source-hash\":\"84316cd62ded95c431d6e6a91cda76565f65491ed180cf702fb630a2c90862b7\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"secret\":{\"type\":\"string\",\"maxLength\":256,\"pattern\":\"^[\\\\u0000-\\\\u00ff]*$\"},\"input_key_id\":{\"type\":\"string\",\"maxLength\":64,\"x-sce-max-bytes\":64},\"salt\":{\"type\":\"string\",\"maxLength\":64,\"pattern\":\"^[\\\\u0000-\\\\u00ff]*$\"},\"info\":{\"type\":\"string\",\"maxLength\":256,\"pattern\":\"^[\\\\u0000-\\\\u00ff]*$\"}},\"required\":[\"secret\",\"input_key_id\",\"salt\",\"info\"],\"additionalProperties\":false}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order, built
/// whole by its initialiser, every field written: no type has a value nobody wrote
/// (docs/adr/0099, section 4). A value, so a host that reads one holds a copy.
public struct KeyDeriveRequestPayload: Equatable, Sendable {
    public var secret: [UInt8]
    public var input_key_id: String
    public var salt: [UInt8]
    public var info: [UInt8]

    public init(secret: [UInt8], input_key_id: String, salt: [UInt8], info: [UInt8]) {
        self.secret = secret
        self.input_key_id = input_key_id
        self.salt = salt
        self.info = info
    }
}
