// SCE-GENERATED — DO NOT EDIT
// source-hash: 87cb1c6919474969c85fe2b7a42a31e32f744518495032cfeb9beb63941cc3ab
// source: union_take_result.scxml
// SCE-MAP: union_take_result.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="union")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

/// A value of `union_take_result`: exactly one case, told apart by its tag `kind`, a value of
/// ``EnumTakeKind``. Two values are equal when they are in one case and their records are equal.
public enum UnionTakeResult: Equatable, Sendable {
    /// The revision of the union: its tag enum's, `enum_take_kind`'s (a union declares none of its own).
    public static let revision: UInt32 = 1

    /// The most bytes a value's JSON text takes: its largest case, every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 50

    /// The id of the union's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:union_take_result:1"

    /// A value as a JSON Schema (2020-12): the `oneOf` of the cases, each its tag `kind` beside the
    /// fields of its record, every bound as this build resolved it.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:union_take_result:1\",\"x-sce-source-hash\":\"87cb1c6919474969c85fe2b7a42a31e32f744518495032cfeb9beb63941cc3ab\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"oneOf\":[{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"taken\"}},\"required\":[\"kind\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"duplicate\"},\"last\":{\"type\":\"string\",\"pattern\":\"^(0|[1-9][0-9]{0,18}|1[0-7][0-9]{18}|18[0-3][0-9]{17}|184[0-3][0-9]{16}|1844[0-5][0-9]{15}|18446[0-6][0-9]{14}|184467[0-3][0-9]{13}|1844674[0-3][0-9]{12}|184467440[0-6][0-9]{10}|1844674407[0-2][0-9]{9}|18446744073[0-6][0-9]{8}|1844674407370[0-8][0-9]{6}|18446744073709[0-4][0-9]{5}|184467440737095[0-4][0-9]{4}|18446744073709550[0-9]{3}|18446744073709551[0-5][0-9]{2}|1844674407370955160[0-9]|1844674407370955161[0-5])$\"}},\"required\":[\"kind\",\"last\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"no_room\"}},\"required\":[\"kind\"],\"additionalProperties\":false}]}"

    /// The case `taken`, its tag alone.
    case taken

    /// The case `duplicate`, holding its record.
    case duplicate(SchemaTakeDuplicatePayload)

    /// The case `no_room`, its tag alone.
    case noRoom

    /// The case this value is in: the tag its text writes under `kind`.
    public var kind: EnumTakeKind {
        switch self {
        case .taken:
            return EnumTakeKind.taken
        case .duplicate:
            return EnumTakeKind.duplicate
        case .noRoom:
            return EnumTakeKind.noRoom
        }
    }

    /// The record of the case `duplicate`, read where a test of the tag proved the value is in it.
    public var provenDuplicate: SchemaTakeDuplicatePayload {
        guard case .duplicate(let value) = self else {
            preconditionFailure("a union_take_result read as duplicate is in \(kind.declaredName)")
        }
        return value
    }
}
