// SCE-GENERATED — DO NOT EDIT
// source-hash: f3a316aacf7403325399aec899293549220a4c98b78ad03825aff15e55acb6a9
// SCE-MAP: union_ast_export_min.scxml:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="union")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

/// A value of `union_ast_export_min`: exactly one case, told apart by its tag `kind`, a value of
/// ``EnumAstExportMin``. Two values are equal when they are in one case and their records are equal.
public enum UnionAstExportMin: Equatable, Sendable {
    /// The revision of the union: its tag enum's, `enum_ast_export_min`'s (a union declares none of its own).
    public static let revision: UInt32 = 1

    /// The most bytes a value's JSON text takes: its largest case, every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 31

    /// The id of the union's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:union_ast_export_min:1"

    /// A value as a JSON Schema (2020-12): the `oneOf` of the cases, each its tag `kind` beside the
    /// fields of its record, every bound as this build resolved it.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:union_ast_export_min:1\",\"x-sce-source-hash\":\"f3a316aacf7403325399aec899293549220a4c98b78ad03825aff15e55acb6a9\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"oneOf\":[{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"a\"},\"payload_byte\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255}},\"required\":[\"kind\",\"payload_byte\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"b\"}},\"required\":[\"kind\"],\"additionalProperties\":false}]}"

    /// The case `a`, holding its record.
    case a(EventSchemaAstExportMinPayload)

    /// The case `b`, its tag alone.
    case b

    /// The case this value is in: the tag its text writes under `kind`.
    public var kind: EnumAstExportMin {
        switch self {
        case .a:
            return EnumAstExportMin.a
        case .b:
            return EnumAstExportMin.b
        }
    }

    /// The record of the case `a`, read where a test of the tag proved the value is in it.
    public var provenA: EventSchemaAstExportMinPayload {
        guard case .a(let value) = self else {
            preconditionFailure("a union_ast_export_min read as a is in \(kind.declaredName)")
        }
        return value
    }
}
