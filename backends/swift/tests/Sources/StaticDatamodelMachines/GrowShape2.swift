// SCE-GENERATED — DO NOT EDIT
// source-hash: ce0c35751fbf2e6199fd3fe9fd7b36ec217e20d7bbd894bd4bbb3bdea73edc6e
// source: grow_shape_2.scxml
// SCE-MAP: grow_shape_2.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="union")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

/// A value of `grow_shape_2`: exactly one case, told apart by its tag `kind`, a value of
/// ``GrowKind``. Two values are equal when they are in one case and their records are equal.
public enum GrowShape2: Equatable, Sendable {
    /// The revision of the union: its tag enum's, `grow_kind`'s (a union declares none of its own).
    public static let revision: UInt32 = 1

    /// The most bytes a value's JSON text takes: its largest case, every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 131

    /// The id of the union's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:grow_shape_2:1"

    /// A value as a JSON Schema (2020-12): the `oneOf` of the cases, each its tag `kind` beside the
    /// fields of its record, every bound as this build resolved it.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:grow_shape_2:1\",\"x-sce-source-hash\":\"ce0c35751fbf2e6199fd3fe9fd7b36ec217e20d7bbd894bd4bbb3bdea73edc6e\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"oneOf\":[{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"single\"},\"n\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535},\"rank\":{\"type\":\"integer\",\"minimum\":-128,\"maximum\":127},\"tags\":{\"type\":\"array\",\"items\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"maxItems\":2},\"leaf\":{\"anyOf\":[{\"$ref\":\"#/$defs/grow_leaf\"},{\"type\":\"null\"}]},\"memo\":{\"anyOf\":[{\"type\":\"string\",\"maxLength\":8,\"x-sce-max-bytes\":8},{\"type\":\"null\"}]}},\"required\":[\"kind\",\"n\",\"rank\",\"tags\",\"leaf\",\"memo\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"none\"}},\"required\":[\"kind\"],\"additionalProperties\":false}],\"$defs\":{\"grow_leaf\":{\"type\":\"object\",\"properties\":{\"v\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255}},\"required\":[\"v\"],\"additionalProperties\":false}}}"

    /// The case `single`, holding its record.
    case single(GrowInner2Payload)

    /// The case `none`, its tag alone.
    case none

    /// The case this value is in: the tag its text writes under `kind`.
    public var kind: GrowKind {
        switch self {
        case .single:
            return GrowKind.single
        case .none:
            return GrowKind.none
        }
    }

    /// The record of the case `single`, read where a test of the tag proved the value is in it.
    public var provenSingle: GrowInner2Payload {
        guard case .single(let value) = self else {
            preconditionFailure("a grow_shape_2 read as single is in \(kind.declaredName)")
        }
        return value
    }
}
