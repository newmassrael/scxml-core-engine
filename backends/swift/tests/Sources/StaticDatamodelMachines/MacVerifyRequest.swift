// SCE-GENERATED — DO NOT EDIT
// source-hash: 5a4f3fa3ef53bf6291db18c5c9453945c685ffb48ac17b60605d76aebf63de19
// source: sce:std/host/mac_verify_request.scxml
// SCE-MAP: mac_verify_request.scxml:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: host.mac.verify.request
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum MacVerifyRequest {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "host.mac.verify.request"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most UTF-8 bytes `key_id` holds (the bound its type writes), as this build resolved it.
    public static let key_idMaxSize: UInt32 = 64

    /// The most bytes `message` holds (the bound its type writes, named `mac_message`), as this build resolved it.
    public static let messageMaxSize: UInt32 = 4096

    /// The most bytes `tag` holds (the bound its type writes), as this build resolved it.
    public static let tagMaxSize: UInt32 = 32

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 25187

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:mac_verify_request:1"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:mac_verify_request:1\",\"x-sce-source-hash\":\"5a4f3fa3ef53bf6291db18c5c9453945c685ffb48ac17b60605d76aebf63de19\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"key_id\":{\"type\":\"string\",\"maxLength\":64,\"x-sce-max-bytes\":64},\"message\":{\"type\":\"string\",\"maxLength\":4096,\"pattern\":\"^[\\\\u0000-\\\\u00ff]*$\"},\"tag\":{\"type\":\"string\",\"maxLength\":32,\"pattern\":\"^[\\\\u0000-\\\\u00ff]*$\"}},\"required\":[\"key_id\",\"message\",\"tag\"],\"additionalProperties\":false}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order, built
/// whole by its initialiser, every field written: no type has a value nobody wrote
/// (docs/adr/0099, section 4). A value, so a host that reads one holds a copy.
public struct MacVerifyRequestPayload: Equatable, Sendable {
    public var key_id: String
    public var message: [UInt8]
    public var tag: [UInt8]

    public init(key_id: String, message: [UInt8], tag: [UInt8]) {
        self.key_id = key_id
        self.message = message
        self.tag = tag
    }
}
