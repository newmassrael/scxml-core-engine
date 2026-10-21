// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//! A payload field a schema declares `int64` or `uint64` is read from the text of
//! its digits, or from a JSON number up to 2^53 - 1 (docs/SCE_ACCEPTED_SUBSET.md
//! §2.15, "A 64-bit integer").
//!
//! A JSON number is a double on an engine that reads it as one, so a 64-bit integer
//! crosses as text; a number past 2^53 - 1 is refused here as it is on every engine,
//! so one payload gives one field wherever it is read. `static_wire_wide` holds the
//! generated machines to the same values; the cases here pin the reader itself.
#![cfg(feature = "std")]

use sce_rust_runtime::event_payload::PayloadFields;

fn signed(text: &str) -> Result<i64, String> {
    let data = format!("{{\"value\": {text}}}");
    PayloadFields::decode(&data)
        .map_err(|e| e.to_string())?
        .signed64("value")
        .map_err(|e| e.to_string())
}

fn unsigned(text: &str) -> Result<u64, String> {
    let data = format!("{{\"value\": {text}}}");
    PayloadFields::decode(&data)
        .map_err(|e| e.to_string())?
        .unsigned64("value")
        .map_err(|e| e.to_string())
}

#[test]
fn the_text_of_the_digits_is_read_exactly_past_2_pow_53() {
    assert_eq!(unsigned(r#""9007199254740993""#), Ok(9_007_199_254_740_993));
    assert_eq!(unsigned(r#""18446744073709551615""#), Ok(u64::MAX));
    assert_eq!(signed(r#""-9007199254740993""#), Ok(-9_007_199_254_740_993));
    assert_eq!(signed(r#""-9223372036854775808""#), Ok(i64::MIN));
    assert_eq!(signed(r#""9223372036854775807""#), Ok(i64::MAX));
}

#[test]
fn a_number_a_double_holds_exactly_is_read_as_a_number() {
    assert_eq!(unsigned("5"), Ok(5));
    assert_eq!(signed("-5"), Ok(-5));
    assert_eq!(unsigned("9007199254740991"), Ok(9_007_199_254_740_991));
    assert_eq!(signed("-9007199254740991"), Ok(-9_007_199_254_740_991));
}

#[test]
fn a_number_past_2_pow_53_minus_1_is_refused_on_every_engine() {
    for text in [
        "9007199254740992",
        "9007199254740993",
        "18446744073709551615",
    ] {
        let refusal = unsigned(text).expect_err("past what a double holds exactly");
        assert!(refusal.contains("past 2^53 - 1"), "{text}: {refusal}");
    }
    let refusal = signed("-9007199254740992").expect_err("past what a double holds exactly");
    assert!(refusal.contains("past 2^53 - 1"), "{refusal}");
}

#[test]
fn digits_that_are_no_canonical_whole_number_are_refused() {
    for text in [
        r#""007""#, r#""-0""#, r#""+3""#, r#""1.5""#, r#""1e3""#, r#""""#, r#"" 1""#,
    ] {
        let refusal = signed(text).expect_err("not canonical digits");
        assert!(
            refusal.contains("not a whole number written as digits"),
            "{text}: {refusal}"
        );
    }
}

#[test]
fn an_unsigned_field_has_no_minus_sign() {
    let refusal = unsigned(r#""-1""#).expect_err("negative text");
    assert!(
        refusal.contains("not a whole number written as digits"),
        "{refusal}"
    );
    assert!(unsigned("-1").is_err());
}

#[test]
fn a_value_past_the_width_is_refused_not_wrapped() {
    for text in [r#""18446744073709551616""#, r#""99999999999999999999""#] {
        let refusal = unsigned(text).expect_err("past uint64");
        assert!(
            refusal.contains("does not fit the width"),
            "{text}: {refusal}"
        );
    }
    let refusal = signed(r#""9223372036854775808""#).expect_err("past int64");
    assert!(refusal.contains("does not fit the width"), "{refusal}");
}

#[test]
fn a_fraction_a_truth_value_and_a_missing_field_are_no_64_bit_integer() {
    assert!(signed("1.5").is_err());
    let refusal = signed("true").expect_err("a truth value");
    assert!(refusal.contains("is not a number"), "{refusal}");
    let fields = PayloadFields::decode(r#"{"other": 1}"#).unwrap();
    assert!(fields.unsigned64("value").is_err());
}
