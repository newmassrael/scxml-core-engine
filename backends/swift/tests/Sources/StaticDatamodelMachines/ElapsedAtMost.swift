// SCE-GENERATED — DO NOT EDIT
// source-hash: 4074a1c08376b8f69f1df6ed1ed13d6d87549a8986b2065c29db9ec9711b6321
// source: elapsed_at_most.scxml
// SCE-MAP: elapsed_at_most.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

/// The test template `elapsed_at_most` (docs/SCE_TEST_PROFILE.md §11.4): a caseless enum of
/// the module, as an algorithm's unit is, so the `Value`, `matches` and `value` of two templates
/// never share a name, and a template another one references is called through its enum.
public enum ElapsedAtMost {
    /// The type the template matches: `uint32`, as the unit of its type declares it.
    public typealias Value = UInt32

    /// Whether `value` matches the template (ES 201 873-1 Annex B.1,
    /// docs/SCE_TEST_PROFILE.md §6.1).
    ///
    /// `param_limit` is the formal parameter `limit`, of type `uint32`.
    public static func matches(_ value: Value, _ param_limit: UInt32) -> Bool {
        return value <= param_limit
    }
}
