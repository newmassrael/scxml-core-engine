// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//! A payload field a schema declares `float32` is the binary32 nearest the
//! number the payload carries (docs/SCE_ACCEPTED_SUBSET.md §2.15, "A 32-bit
//! real").
//!
//! A JSON number is a double wherever it is read, so every engine rounds once,
//! from that double, and a number past the largest single does not fit the
//! field, as a whole number past its width does not. `static_record_real32`
//! holds the generated machine to the same numbers; the cases here pin the
//! reader itself.
#![cfg(not(feature = "no_std"))]

use sce_rust_runtime::event_payload::PayloadFields;

fn read(text: &str) -> Result<f32, String> {
    let data = format!("{{\"value\": {text}}}");
    PayloadFields::decode(&data)
        .map_err(|e| e.to_string())?
        .float32("value")
        .map_err(|e| e.to_string())
}

#[test]
fn a_number_lands_as_the_binary32_nearest_it() {
    assert_eq!(read("0.1"), Ok(0.1_f32));
    assert_eq!(f64::from(read("0.1").unwrap()), 0.100_000_001_490_116_12);
}

#[test]
fn a_whole_number_is_read_as_the_real_it_is() {
    assert_eq!(read("3"), Ok(3.0_f32));
}

#[test]
fn the_largest_single_fits() {
    // The double the shortest spelling of the largest single reads as.
    assert_eq!(read("3.4028234663852886e38"), Ok(f32::MAX));
}

#[test]
fn a_number_past_the_largest_single_does_not_fit_the_field() {
    for text in ["1e39", "-1e39", "3.5e38"] {
        let refusal = read(text).expect_err("past the range of a single");
        assert!(
            refusal.contains("does not fit the width its schema declares"),
            "{text}: {refusal}"
        );
    }
}

#[test]
fn a_truth_value_is_not_a_number() {
    let data = r#"{"value": true}"#;
    let refusal = PayloadFields::decode(data)
        .unwrap()
        .float32("value")
        .expect_err("a truth value");
    assert!(refusal.reason().contains("is not a number"), "{refusal}");
}
