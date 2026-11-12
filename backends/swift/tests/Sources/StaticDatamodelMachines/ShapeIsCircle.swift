// SCE-GENERATED — DO NOT EDIT
// source-hash: 4e65a893782852676bb9b2d81e2da91064cf31e9245b09ee7b72fd1a04b1ed23
// SCE-MAP: shape_is_circle.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

/// The test template `shape_is_circle` (docs/SCE_TEST_PROFILE.md §11.4): a caseless enum of
/// the module, as an algorithm's unit is, so the `Value`, `matches` and `value` of two templates
/// never share a name, and a template another one references is called through its enum.
public enum ShapeIsCircle {
    /// The type the template matches: `union:Shape`, as the unit of its type declares it.
    public typealias Value = UnionShapeMinimal

    /// Whether `value` matches the template (ES 201 873-1 Annex B.1,
    /// docs/SCE_TEST_PROFILE.md §6.1).
    public static func matches(_ value: Value) -> Bool {
        return value.kind == ShapeKind.circle
    }
}
