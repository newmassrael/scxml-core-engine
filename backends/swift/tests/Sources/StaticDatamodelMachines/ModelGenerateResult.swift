// SCE-GENERATED — DO NOT EDIT
// source-hash: 1f01cd741ca0d625188a4b5d3fc338154b34cfdf32194edda139c3bc09e0016b
// SCE-MAP: model_generate_result.scxml:17 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: host.model.generate.result
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum ModelGenerateResult {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "host.model.generate.result"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most UTF-8 bytes `error` holds (the bound its type writes), as this build resolved it.
    public static let errorMaxSize: UInt32 = 24

    /// The most UTF-8 bytes `text` holds (the bound its type writes, named `model_answer`), as this build resolved it.
    public static let textMaxSize: UInt32 = 4096

    /// The most UTF-8 bytes `model` holds (the bound its type writes), as this build resolved it.
    public static let modelMaxSize: UInt32 = 128

    /// The most UTF-8 bytes `finish` holds (the bound its type writes), as this build resolved it.
    public static let finishMaxSize: UInt32 = 16

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 25703

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:model_generate_result:1"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:model_generate_result:1\",\"x-sce-source-hash\":\"1f01cd741ca0d625188a4b5d3fc338154b34cfdf32194edda139c3bc09e0016b\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"error\":{\"type\":\"string\",\"maxLength\":24,\"x-sce-max-bytes\":24},\"text\":{\"type\":\"string\",\"maxLength\":4096,\"x-sce-max-bytes\":4096},\"constrained\":{\"type\":\"boolean\"},\"model\":{\"type\":\"string\",\"maxLength\":128,\"x-sce-max-bytes\":128},\"prompt_tokens\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"answer_tokens\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"finish\":{\"type\":\"string\",\"maxLength\":16,\"x-sce-max-bytes\":16}},\"required\":[\"error\",\"text\",\"constrained\",\"model\",\"prompt_tokens\",\"answer_tokens\",\"finish\"],\"additionalProperties\":false}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order, built
/// whole by its initialiser, every field written: no type has a value nobody wrote
/// (docs/adr/0098, section 4). A value, so a host that reads one holds a copy.
public struct ModelGenerateResultPayload: Equatable, Sendable {
    public var error: String
    public var text: String
    public var constrained: Bool
    public var model: String
    public var prompt_tokens: UInt32
    public var answer_tokens: UInt32
    public var finish: String

    public init(error: String, text: String, constrained: Bool, model: String, prompt_tokens: UInt32, answer_tokens: UInt32, finish: String) {
        self.error = error
        self.text = text
        self.constrained = constrained
        self.model = model
        self.prompt_tokens = prompt_tokens
        self.answer_tokens = answer_tokens
        self.finish = finish
    }
}
