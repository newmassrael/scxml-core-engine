// SCE-GENERATED — DO NOT EDIT
// source-hash: 094be67405b37e3736355b0f2f05513c7be48c3e316ec6c711a7d6e72dc4648a
// source: sce:std/host/key_generate_request.scxml
// SCE-MAP: key_generate_request.scxml:14 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: host.key.generate.request
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum KeyGenerateRequest {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "host.key.generate.request"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most UTF-8 bytes `label` holds (the bound its type writes), as this build resolved it.
    public static let labelMaxSize: UInt32 = 64

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 415

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:key_generate_request:1"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:key_generate_request:1\",\"x-sce-source-hash\":\"094be67405b37e3736355b0f2f05513c7be48c3e316ec6c711a7d6e72dc4648a\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"key_use\":{\"enum\":[\"sign\",\"seal\",\"mac\",\"derive\"]},\"label\":{\"type\":\"string\",\"maxLength\":64,\"x-sce-max-bytes\":64}},\"required\":[\"key_use\",\"label\"],\"additionalProperties\":false}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order, built
/// whole by its initialiser, every field written: no type has a value nobody wrote
/// (docs/adr/0099, section 4). A value, so a host that reads one holds a copy.
public struct KeyGenerateRequestPayload: Equatable, Sendable {
    public var key_use: KeyUse
    public var label: String

    public init(key_use: KeyUse, label: String) {
        self.key_use = key_use
        self.label = label
    }
}
