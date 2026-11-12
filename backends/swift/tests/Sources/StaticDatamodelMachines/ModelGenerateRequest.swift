// SCE-GENERATED — DO NOT EDIT
// source-hash: 1f01cd741ca0d625188a4b5d3fc338154b34cfdf32194edda139c3bc09e0016b
// SCE-MAP: model_generate_request.scxml:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: host.model.generate.request
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum ModelGenerateRequest {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "host.model.generate.request"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most UTF-8 bytes `prompt` holds (the bound its type writes, named `model_prompt`), as this build resolved it.
    public static let promptMaxSize: UInt32 = 8192

    /// The most UTF-8 bytes `answer_schema` holds (the bound its type writes), as this build resolved it.
    public static let answer_schemaMaxSize: UInt32 = 128

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 50002

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:model_generate_request:1"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:model_generate_request:1\",\"x-sce-source-hash\":\"1f01cd741ca0d625188a4b5d3fc338154b34cfdf32194edda139c3bc09e0016b\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"prompt\":{\"type\":\"string\",\"maxLength\":8192,\"x-sce-max-bytes\":8192},\"answer_schema\":{\"type\":\"string\",\"maxLength\":128,\"x-sce-max-bytes\":128},\"max_tokens\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"temperature_milli\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"prompt\",\"answer_schema\",\"max_tokens\",\"temperature_milli\"],\"additionalProperties\":false}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order, built
/// whole by its initialiser, every field written: no type has a value nobody wrote
/// (docs/adr/0098, section 4). A value, so a host that reads one holds a copy.
public struct ModelGenerateRequestPayload: Equatable, Sendable {
    public var prompt: String
    public var answer_schema: String
    public var max_tokens: UInt32
    public var temperature_milli: UInt16

    public init(prompt: String, answer_schema: String, max_tokens: UInt32, temperature_milli: UInt16) {
        self.prompt = prompt
        self.answer_schema = answer_schema
        self.max_tokens = max_tokens
        self.temperature_milli = temperature_milli
    }
}
