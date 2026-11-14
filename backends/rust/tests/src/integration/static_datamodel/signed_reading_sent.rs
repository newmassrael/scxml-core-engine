// SCE-GENERATED — DO NOT EDIT
// source-hash: 242afb729b9865bf6d727ab9bf3b98d23b9e6f9c59d9d146a0381a194dfbe13b
#![doc = "SCE-MAP: signed_reading_sent.scxml:5 :: _forge_body"]
// SCE-MAP: signed_reading_sent.scxml:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

/// The type the template `signed_reading_sent` matches: `record:SignedReading`, as the
/// unit of its type declares it.
#[allow(dead_code)]
pub type Value = super::signed_reading::SignedReadingPayload;

/// Whether `value` matches the template `signed_reading_sent` (ES 201 873-1 Annex B.1,
/// docs/SCE_TEST_PROFILE.md §6.1).
#[allow(dead_code, unused_variables)]
pub fn matches(value: &Value) -> bool {
    value.delta == -7
        && value.offset == -300
        && value.level == -70000
        && value.stamp == -5000000000
        && value.total == 10000000000000000000
}

/// The value the specific template `signed_reading_sent` sends (docs/SCE_TEST_PROFILE.md
/// §6.2): every field it names at its value, every field it omits absent. It matches the
/// template by `matches`, given the same parameters.
#[allow(dead_code, unused_variables)]
pub fn value() -> Value {
    super::signed_reading::SignedReadingPayload {
        delta: -7,
        offset: -300,
        level: -70000,
        stamp: -5000000000,
        total: 10000000000000000000,
    }
}

/// The name of the record `Value` is, as the types module of the suite's TTCN-3 export names it
/// (docs/SCE_TEST_PROFILE.md §9.5): the `type` of a message of it in the test log (§12.5).
#[allow(dead_code)]
pub const TYPE: &str = "signed_reading";

/// `value` in the canonical form of the test log (docs/SCE_TEST_PROFILE.md §12.5): each
/// integer as the text of its digits, a byte string in upper-case hex, an enum and a case
/// of a union by their declared names, an absent field omitted. A send of this template
/// carries it beside the value it sends (`sce_rust_runtime::test::message_value`).
#[allow(dead_code)]
pub fn canonical(value: &Value) -> ::sce_rust_runtime::json::Value {
    ::sce_rust_runtime::test::canonical::record([
        (
            "delta",
            Some(::sce_rust_runtime::test::canonical::integer(value.delta)),
        ),
        (
            "offset",
            Some(::sce_rust_runtime::test::canonical::integer(value.offset)),
        ),
        (
            "level",
            Some(::sce_rust_runtime::test::canonical::integer(value.level)),
        ),
        (
            "stamp",
            Some(::sce_rust_runtime::test::canonical::integer(value.stamp)),
        ),
        (
            "total",
            Some(::sce_rust_runtime::test::canonical::integer(value.total)),
        ),
    ])
}
