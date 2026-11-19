// SCE-GENERATED — DO NOT EDIT
// source-hash: 45260521b7a1b8de46f9c0511b40e9fa6c3287c2c89986939b7f90cc4a9dd513
// source: sce:std/host/kv_get_result.scxml
// SCE-MAP: kv_get_result.scxml:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: host.kv.get.result
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum KvGetResult {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "host.kv.get.result"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most UTF-8 bytes `error` holds (the bound its type writes), as this build resolved it.
    public static let errorMaxSize: UInt32 = 24

    /// The most UTF-8 bytes `value` holds (the bound its type writes), as this build resolved it.
    public static let valueMaxSize: UInt32 = 4096

    /// The most UTF-8 bytes `version` holds (the bound its type writes), as this build resolved it.
    public static let versionMaxSize: UInt32 = 20

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 24890

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:kv_get_result:1"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:kv_get_result:1\",\"x-sce-source-hash\":\"45260521b7a1b8de46f9c0511b40e9fa6c3287c2c89986939b7f90cc4a9dd513\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"error\":{\"type\":\"string\",\"maxLength\":24,\"x-sce-max-bytes\":24},\"found\":{\"type\":\"boolean\"},\"value\":{\"type\":\"string\",\"maxLength\":4096,\"x-sce-max-bytes\":4096},\"version\":{\"type\":\"string\",\"maxLength\":20,\"x-sce-max-bytes\":20}},\"required\":[\"error\",\"found\",\"value\",\"version\"],\"additionalProperties\":false}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order, built
/// whole by its initialiser, every field written: no type has a value nobody wrote
/// (docs/adr/0099, section 4). A value, so a host that reads one holds a copy.
public struct KvGetResultPayload: Equatable, Sendable {
    public var error: String
    public var found: Bool
    public var value: String
    public var version: String

    public init(error: String, found: Bool, value: String, version: String) {
        self.error = error
        self.found = found
        self.value = value
        self.version = version
    }
}
