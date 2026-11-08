// SCE-GENERATED — DO NOT EDIT
// source-hash: d4646eb3233d0326b8220cebf3753ec7bf14af5b5faee8ffa467c7536202f42b
// SCE-MAP: card_within.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceTestProfile

/// The test template `card_within` (docs/SCE_TEST_PROFILE.md §11.4): a caseless enum of
/// the module, as an algorithm's unit is, so the `Value`, `matches` and `value` of two templates
/// never share a name, and a template another one references is called through its enum.
public enum CardWithin {
    /// The type the template matches: `record:Card`, as the unit of its type declares it.
    public typealias Value = CardPaymentPayload

    /// Whether `value` matches the template (ES 201 873-1 Annex B.1,
    /// docs/SCE_TEST_PROFILE.md §6.1).
    ///
    /// `param_most` is the formal parameter `most`, of type `uint32`.
    public static func matches(_ value: Value, _ param_most: UInt32) -> Bool {
        return SceTestProfile.textLength(value.last4) >= 2 && SceTestProfile.textLength(value.last4) <= 3 && value.amount >= 1 && value.amount <= param_most
    }
}
