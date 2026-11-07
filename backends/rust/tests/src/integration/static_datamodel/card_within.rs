// SCE-GENERATED — DO NOT EDIT
// source-hash: d4646eb3233d0326b8220cebf3753ec7bf14af5b5faee8ffa467c7536202f42b
#![doc = "SCE-MAP: card_within.scxml:6 :: _forge_body"]
// SCE-MAP: card_within.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

/// The type the template `card_within` matches: `record:Card`, as the
/// unit of its type declares it.
#[allow(dead_code)]
pub type Value = super::card_payment::CardPaymentPayload;

/// Whether `value` matches the template `card_within` (ES 201 873-1 Annex B.1,
/// docs/SCE_TEST_PROFILE.md §6.1).
/// - `param_most`: the formal parameter `most`, of type `uint32`.
#[allow(dead_code, unused_variables)]
pub fn matches(value: &Value, param_most: u32) -> bool {
    value.last4.chars().count() >= 2
        && value.last4.chars().count() <= 3
        && value.amount >= 1
        && value.amount <= param_most
}
