// SCE-GENERATED — DO NOT EDIT
// source-hash: 84fb9ab720a733e5ae584f0097d5e6a562408447dcdd1ab7923585e9289913d3
// SCE-MAP: refusal.scxml:22 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: lift.refusal
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum Refusal {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "lift.refusal"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most UTF-8 bytes `path` holds (the bound its type writes, named `lift_path`), as this build resolved it.
    public static let pathMaxSize: UInt32 = 256

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 1591

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:refusal:1"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:refusal:1\",\"x-sce-source-hash\":\"84fb9ab720a733e5ae584f0097d5e6a562408447dcdd1ab7923585e9289913d3\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"code\":{\"enum\":[\"notJson\",\"notObject\",\"tooDeep\",\"missing\",\"nullNotOptional\",\"wrongKind\",\"badNumber\",\"outOfRange\",\"tooLong\",\"tooMany\",\"unknownVariant\",\"notByteText\",\"nulInText\",\"tooFew\"]},\"path\":{\"type\":\"string\",\"maxLength\":256,\"x-sce-max-bytes\":256},\"bound\":{\"anyOf\":[{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},{\"type\":\"null\"}]}},\"required\":[\"code\",\"path\",\"bound\"],\"additionalProperties\":false}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order, built
/// whole by its initialiser, every field written: no type has a value nobody wrote
/// (docs/adr/0098, section 4). A value, so a host that reads one holds a copy.
public struct RefusalPayload: Equatable, Sendable {
    public var code: RefusalCode
    public var path: String
    public var bound: UInt32?

    public init(code: RefusalCode, path: String, bound: UInt32?) {
        self.code = code
        self.path = path
        self.bound = bound
    }
}
