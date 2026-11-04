// SCE-GENERATED — DO NOT EDIT
// source-hash: c6d298fef1f83f174583c4341b7bfe9193a50cbfa46719e4a4c7f84e6292e6c4
// SCE-MAP: refusal.scxml:21 :: _forge_body

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
    public static let jsonSchema: String = "{\"$id\":\"urn:sce:schema:refusal:1\",\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"additionalProperties\":false,\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"properties\":{\"bound\":{\"anyOf\":[{\"maximum\":4294967295,\"minimum\":0,\"type\":\"integer\"},{\"type\":\"null\"}]},\"code\":{\"enum\":[\"notJson\",\"notObject\",\"tooDeep\",\"missing\",\"nullNotOptional\",\"wrongKind\",\"badNumber\",\"outOfRange\",\"tooLong\",\"tooMany\",\"unknownVariant\",\"notByteText\",\"nulInText\"]},\"path\":{\"maxLength\":256,\"type\":\"string\",\"x-sce-max-bytes\":256}},\"required\":[\"bound\",\"code\",\"path\"],\"type\":\"object\",\"x-sce-source-hash\":\"c6d298fef1f83f174583c4341b7bfe9193a50cbfa46719e4a4c7f84e6292e6c4\"}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct RefusalPayload: Equatable, Sendable, SceDefault {
    public var code: RefusalCode = RefusalCode.allCases[0]
    public var path: String = ""
    public var bound: UInt32? = nil

    public init() {}

    public init(code: RefusalCode, path: String, bound: UInt32?) {
        self.code = code
        self.path = path
        self.bound = bound
    }
}
