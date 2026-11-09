// SCE-GENERATED — DO NOT EDIT
// source-hash: d1761e5ce3c848ac60422dc2a65a21127a26aba980f944059bcd0ab0b5050c1f
// SCE-MAP: union_shape_minimal.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="union")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

/// A value of `union_shape_minimal`: exactly one case, told apart by its tag `kind`, a value of
/// ``ShapeKind``. Two values are equal when they are in one case and their records are equal.
public enum UnionShapeMinimal: Equatable, Sendable {
    /// The revision of the union: its tag enum's, `shape_kind`'s (a union declares none of its own).
    public static let revision: UInt32 = 1

    /// The most bytes a value's JSON text takes: its largest case, every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 44

    /// The id of the union's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:union_shape_minimal:1"

    /// A value as a JSON Schema (2020-12): the `oneOf` of the cases, each its tag `kind` beside the
    /// fields of its record, every bound as this build resolved it.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:union_shape_minimal:1\",\"x-sce-source-hash\":\"d1761e5ce3c848ac60422dc2a65a21127a26aba980f944059bcd0ab0b5050c1f\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"oneOf\":[{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"dot\"}},\"required\":[\"kind\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"circle\"},\"radius\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"kind\",\"radius\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"rect\"},\"width\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535},\"height\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"kind\",\"width\",\"height\"],\"additionalProperties\":false}]}"

    /// The case `dot`, its tag alone.
    case dot

    /// The case `circle`, holding its record.
    case circle(ShapeRadiusMinimalPayload)

    /// The case `rect`, holding its record.
    case rect(ShapeRectMinimalPayload)

    /// The case this value is in: the tag its text writes under `kind`.
    public var kind: ShapeKind {
        switch self {
        case .dot:
            return ShapeKind.dot
        case .circle:
            return ShapeKind.circle
        case .rect:
            return ShapeKind.rect
        }
    }

    /// The record of the case `circle`, read where a test of the tag proved the value is in it.
    public var provenCircle: ShapeRadiusMinimalPayload {
        guard case .circle(let value) = self else {
            preconditionFailure("a union_shape_minimal read as circle is in \(kind.declaredName)")
        }
        return value
    }

    /// The record of the case `rect`, read where a test of the tag proved the value is in it.
    public var provenRect: ShapeRectMinimalPayload {
        guard case .rect(let value) = self else {
            preconditionFailure("a union_shape_minimal read as rect is in \(kind.declaredName)")
        }
        return value
    }
}
