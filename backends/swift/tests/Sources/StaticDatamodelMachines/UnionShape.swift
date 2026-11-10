// SCE-GENERATED — DO NOT EDIT
// source-hash: d8c75e4d0233623996dc9a7cc3bdbe850af8d4f91ab023212ebd9f938cf7955f
// SCE-MAP: union_shape.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="union")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

/// A value of `union_shape`: exactly one case, told apart by its tag `kind`, a value of
/// ``EnumShapeKind``. Two values are equal when they are in one case and their records are equal.
public enum UnionShape: Equatable, Sendable {
    /// The revision of the union: its tag enum's, `enum_shape_kind`'s (a union declares none of its own).
    public static let revision: UInt32 = 1

    /// The most bytes a value's JSON text takes: its largest case, every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 108

    /// The id of the union's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:union_shape:1"

    /// A value as a JSON Schema (2020-12): the `oneOf` of the cases, each its tag `kind` beside the
    /// fields of its record, every bound as this build resolved it.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:union_shape:1\",\"x-sce-source-hash\":\"d8c75e4d0233623996dc9a7cc3bdbe850af8d4f91ab023212ebd9f938cf7955f\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"oneOf\":[{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"dot\"}},\"required\":[\"kind\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"circle\"},\"radius\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"kind\",\"radius\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"disc\"},\"radius\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"kind\",\"radius\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"rect\"},\"width\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535},\"height\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"kind\",\"width\",\"height\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"label\"},\"text\":{\"type\":\"string\",\"maxLength\":8,\"x-sce-max-bytes\":8},\"note\":{\"anyOf\":[{\"type\":\"string\",\"maxLength\":4,\"x-sce-max-bytes\":4},{\"type\":\"null\"}]}},\"required\":[\"kind\",\"text\",\"note\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"path\"},\"points\":{\"type\":\"array\",\"items\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"maxItems\":3}},\"required\":[\"kind\",\"points\"],\"additionalProperties\":false}]}"

    /// The case `dot`, its tag alone.
    case dot

    /// The case `circle`, holding its record.
    case circle(ShapeRadiusPayload)

    /// The case `disc`, holding its record.
    case disc(ShapeRadiusPayload)

    /// The case `rect`, holding its record.
    case rect(ShapeRectPayload)

    /// The case `label`, holding its record.
    case label(ShapeLabelPayload)

    /// The case `path`, holding its record.
    case path(ShapePathPayload)

    /// The case this value is in: the tag its text writes under `kind`.
    public var kind: EnumShapeKind {
        switch self {
        case .dot:
            return EnumShapeKind.dot
        case .circle:
            return EnumShapeKind.circle
        case .disc:
            return EnumShapeKind.disc
        case .rect:
            return EnumShapeKind.rect
        case .label:
            return EnumShapeKind.label
        case .path:
            return EnumShapeKind.path
        }
    }

    /// The record of the case `circle`, read where a test of the tag proved the value is in it.
    public var provenCircle: ShapeRadiusPayload {
        guard case .circle(let value) = self else {
            preconditionFailure("a union_shape read as circle is in \(kind.declaredName)")
        }
        return value
    }

    /// The record of the case `disc`, read where a test of the tag proved the value is in it.
    public var provenDisc: ShapeRadiusPayload {
        guard case .disc(let value) = self else {
            preconditionFailure("a union_shape read as disc is in \(kind.declaredName)")
        }
        return value
    }

    /// The record of the case `rect`, read where a test of the tag proved the value is in it.
    public var provenRect: ShapeRectPayload {
        guard case .rect(let value) = self else {
            preconditionFailure("a union_shape read as rect is in \(kind.declaredName)")
        }
        return value
    }

    /// The record of the case `label`, read where a test of the tag proved the value is in it.
    public var provenLabel: ShapeLabelPayload {
        guard case .label(let value) = self else {
            preconditionFailure("a union_shape read as label is in \(kind.declaredName)")
        }
        return value
    }

    /// The record of the case `path`, read where a test of the tag proved the value is in it.
    public var provenPath: ShapePathPayload {
        guard case .path(let value) = self else {
            preconditionFailure("a union_shape read as path is in \(kind.declaredName)")
        }
        return value
    }
}
