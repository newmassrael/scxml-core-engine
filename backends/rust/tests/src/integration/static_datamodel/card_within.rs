// SCE-GENERATED — DO NOT EDIT
// source-hash: d4646eb3233d0326b8220cebf3753ec7bf14af5b5faee8ffa467c7536202f42b
#![doc = "SCE-MAP: card_within.scxml:6 :: _forge_body"]
// SCE-MAP: card_within.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: sce_runtime
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

/// The name of the record `Value` is, as the types module of the suite's TTCN-3 export names it
/// (docs/SCE_TEST_PROFILE.md §9.5): the `type` of a message of it in the test log (§12.5).
#[allow(dead_code)]
pub const TYPE: &str = "card_payment";

/// `value` in the canonical form of the test log (docs/SCE_TEST_PROFILE.md §12.5): each
/// integer as the text of its digits, a byte string in upper-case hex, an enum and a case
/// of a union by their declared names, an absent field omitted. A send of this template
/// carries it beside the value it sends (`sce_rust_runtime::test::message_value`).
#[allow(dead_code)]
pub fn canonical(value: &Value) -> ::sce_rust_runtime::json::Value {
    ::sce_rust_runtime::test::canonical::record([
        (
            "last4",
            Some(::sce_rust_runtime::test::canonical::text(
                value.last4.as_str(),
            )),
        ),
        (
            "amount",
            Some(::sce_rust_runtime::test::canonical::integer(value.amount)),
        ),
    ])
}
