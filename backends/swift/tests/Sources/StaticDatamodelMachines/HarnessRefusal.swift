// SCE-GENERATED — DO NOT EDIT
// source-hash: 9c4f25c71477b8168828f3c8f9ccad85a4253ff428db93b5b34a11bbacc1faa1
// SCE-MAP: harness_refusal.scxml:19 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: harness.refusal
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum HarnessRefusal {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "harness.refusal"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most UTF-8 bytes `path` holds (the bound its type writes), as this build resolved it.
    public static let pathMaxSize: UInt32 = 256

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 1572

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:harness_refusal:1"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:harness_refusal:1\",\"x-sce-source-hash\":\"9c4f25c71477b8168828f3c8f9ccad85a4253ff428db93b5b34a11bbacc1faa1\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"code\":{\"enum\":[\"notJson\",\"notObject\",\"tooDeep\",\"missing\",\"nullNotOptional\",\"wrongKind\",\"badNumber\",\"outOfRange\",\"tooLong\",\"tooMany\",\"unknownVariant\",\"notByteText\",\"nulInText\",\"tooFew\",\"notOffered\"]},\"path\":{\"type\":\"string\",\"maxLength\":256,\"x-sce-max-bytes\":256}},\"required\":[\"code\",\"path\"],\"additionalProperties\":false}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct HarnessRefusalPayload: Equatable, Sendable, SceDefault {
    public var code: EnumHarnessCode = EnumHarnessCode.allCases[0]
    public var path: String = ""

    public init() {}

    public init(code: EnumHarnessCode, path: String) {
        self.code = code
        self.path = path
    }
}
