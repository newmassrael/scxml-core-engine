// SCE-GENERATED — DO NOT EDIT
// source-hash: 792468e33f930b682d12af1a48545cd870803e0bcabb117eb4a2299af5ef6f2b
// source: sce:std/host/kv_put_if_request.scxml
// SCE-MAP: kv_put_if_request.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: host.kv.put_if.request
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum KvPutIfRequest {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "host.kv.put_if.request"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most UTF-8 bytes `key` holds (the bound its type writes), as this build resolved it.
    public static let keyMaxSize: UInt32 = 256

    /// The most UTF-8 bytes `value` holds (the bound its type writes), as this build resolved it.
    public static let valueMaxSize: UInt32 = 4096

    /// The most UTF-8 bytes `expected_version` holds (the bound its type writes), as this build resolved it.
    public static let expected_versionMaxSize: UInt32 = 20

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 26275

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:kv_put_if_request:1"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:kv_put_if_request:1\",\"x-sce-source-hash\":\"792468e33f930b682d12af1a48545cd870803e0bcabb117eb4a2299af5ef6f2b\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"key\":{\"type\":\"string\",\"maxLength\":256,\"x-sce-max-bytes\":256},\"value\":{\"type\":\"string\",\"maxLength\":4096,\"x-sce-max-bytes\":4096},\"expected_version\":{\"type\":\"string\",\"maxLength\":20,\"x-sce-max-bytes\":20}},\"required\":[\"key\",\"value\",\"expected_version\"],\"additionalProperties\":false}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order, built
/// whole by its initialiser, every field written: no type has a value nobody wrote
/// (docs/adr/0099, section 4). A value, so a host that reads one holds a copy.
public struct KvPutIfRequestPayload: Equatable, Sendable {
    public var key: String
    public var value: String
    public var expected_version: String

    public init(key: String, value: String, expected_version: String) {
        self.key = key
        self.value = value
        self.expected_version = expected_version
    }
}
