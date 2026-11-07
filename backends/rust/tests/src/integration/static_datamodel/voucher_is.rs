// SCE-GENERATED — DO NOT EDIT
// source-hash: d4646eb3233d0326b8220cebf3753ec7bf14af5b5faee8ffa467c7536202f42b
#![doc = "SCE-MAP: voucher_is.scxml:4 :: _forge_body"]
// SCE-MAP: voucher_is.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

/// The type the template `voucher_is` matches: `record:Voucher`, as the
/// unit of its type declares it.
#[allow(dead_code)]
pub type Value = super::voucher_payment::VoucherPaymentPayload;

/// Whether `value` matches the template `voucher_is` (ES 201 873-1 Annex B.1,
/// docs/SCE_TEST_PROFILE.md §6.1).
#[allow(dead_code, unused_variables)]
pub fn matches(value: &Value) -> bool {
    value.voucher_code[..] == b"\x00\xff\x10\xa0"[..]
}

/// The value the specific template `voucher_is` sends (docs/SCE_TEST_PROFILE.md
/// §6.2): every field it names at its value, every field it omits absent. It matches the
/// template by `matches`, given the same parameters.
#[allow(dead_code, unused_variables)]
pub fn value() -> Value {
    super::voucher_payment::VoucherPaymentPayload {
        voucher_code: b"\x00\xff\x10\xa0".to_vec(),
    }
}
