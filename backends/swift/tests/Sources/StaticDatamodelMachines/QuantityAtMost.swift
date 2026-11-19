// SCE-GENERATED — DO NOT EDIT
// source-hash: 29d573c8acf7836d816f1ff0ec9e6833df3bbd5696937f47350d29d8ef13b2f9
// source: quantity_at_most.scxml
// SCE-MAP: quantity_at_most.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

/// The test template `quantity_at_most` (docs/SCE_TEST_PROFILE.md §11.4): a caseless enum of
/// the module, as an algorithm's unit is, so the `Value`, `matches` and `value` of two templates
/// never share a name, and a template another one references is called through its enum.
public enum QuantityAtMost {
    /// The type the template matches: `uint16`, as the unit of its type declares it.
    public typealias Value = UInt16

    /// Whether `value` matches the template (ES 201 873-1 Annex B.1,
    /// docs/SCE_TEST_PROFILE.md §6.1).
    ///
    /// `param_most` is the formal parameter `most`, of type `uint16`.
    public static func matches(_ value: Value, _ param_most: UInt16) -> Bool {
        return value <= param_most
    }
}
