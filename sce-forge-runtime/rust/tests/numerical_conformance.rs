// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Cross-language numerical conformance harness (Rust half).
//
// The fixtures included below are generated at build time by build.rs, which
// calls sce_build::compile_forge_with_imports_validated() on the SCXML files
// in tests/forge/resources/. They are written to OUT_DIR and pulled in here
// via include!. No committed Rust goldens are consumed — the single source of
// truth is the SCXML and the codegen library.
//
// This integration test then runs the generated fixtures against the
// reference vectors in tests/forge/conformance/numerical_reference.json. The
// same JSON file is consumed by the C++, Python, Kotlin, and Go conformance
// tests; any drift between languages surfaces as a failure in whichever
// language is wrong.

#[allow(dead_code, non_snake_case)]
mod fixtures {
    pub mod interpolation_1d_linear {
        include!(concat!(env!("OUT_DIR"), "/interpolation_1d_linear.rs"));
    }
    pub mod interpolation_2d_bilinear {
        include!(concat!(env!("OUT_DIR"), "/interpolation_2d_bilinear.rs"));
    }
    pub mod filter_moving_average {
        include!(concat!(env!("OUT_DIR"), "/filter_moving_average.rs"));
    }
    pub mod filter_debounce {
        include!(concat!(env!("OUT_DIR"), "/filter_debounce.rs"));
    }
    pub mod observer_coolant {
        include!(concat!(env!("OUT_DIR"), "/observer_coolant.rs"));
    }
}

use fixtures::filter_debounce::FilterDebounce;
use fixtures::filter_moving_average::FilterMovingAverage;
use fixtures::interpolation_1d_linear::Interpolation1dLinear;
use fixtures::interpolation_2d_bilinear::Interpolation2dBilinear;
use fixtures::observer_coolant::ObserverCoolant;

fn load_reference() -> serde_json::Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/forge/conformance/numerical_reference.json"
    );
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read {path}: {e}"));
    serde_json::from_str(&text).expect("reference JSON is not valid")
}

fn tolerance(reference: &serde_json::Value) -> f64 {
    reference["float_tolerance"]
        .as_f64()
        .expect("float_tolerance must be a number")
}

fn assert_close(actual: f64, expected: f64, tol: f64, label: &str) {
    let diff = (actual - expected).abs();
    assert!(
        diff <= tol,
        "{label}: actual={actual}, expected={expected}, diff={diff}, tol={tol}"
    );
}

#[test]
fn interpolation_1d_linear_matches_reference() {
    let reference = load_reference();
    let tol = tolerance(&reference);
    let spec = &reference["pure_functions"]["interpolation_1d_linear"];
    for case in spec["cases"].as_array().expect("cases must be array") {
        let rpm = case["args"][0].as_u64().expect("rpm is integer") as u16;
        let expected = case["expected"].as_f64().expect("expected is number");
        let actual = Interpolation1dLinear::lookup(rpm);
        assert_close(
            actual,
            expected,
            tol,
            &format!("interpolation_1d_linear({rpm})"),
        );
    }
}

#[test]
fn interpolation_2d_bilinear_matches_reference() {
    let reference = load_reference();
    let tol = tolerance(&reference);
    let spec = &reference["pure_functions"]["interpolation_2d_bilinear"];
    for case in spec["cases"].as_array().expect("cases must be array") {
        let rpm = case["args"][0].as_u64().expect("rpm is integer") as u16;
        let load = case["args"][1].as_u64().expect("load is integer") as u8;
        let expected = case["expected"].as_f64().expect("expected is number");
        let actual = Interpolation2dBilinear::lookup(rpm, load);
        assert_close(
            actual,
            expected,
            tol,
            &format!("interpolation_2d_bilinear({rpm}, {load})"),
        );
    }
}

#[test]
fn filter_moving_average_matches_reference() {
    let reference = load_reference();
    let tol = tolerance(&reference);
    let spec = &reference["stateful_filters"]["filter_moving_average"];
    let mut filter = FilterMovingAverage::new();
    for (i, step) in spec["sequence"]
        .as_array()
        .expect("sequence must be array")
        .iter()
        .enumerate()
    {
        let input = step["input"].as_f64().expect("input is number");
        let expected = step["expected"].as_f64().expect("expected is number");
        let actual = filter.update(input);
        assert_close(
            actual,
            expected,
            tol,
            &format!("filter_moving_average step {i} input={input}"),
        );
    }
}

#[test]
fn filter_debounce_matches_reference() {
    let reference = load_reference();
    let spec = &reference["stateful_filters"]["filter_debounce"];
    let mut filter = FilterDebounce::new();
    for (i, step) in spec["sequence"]
        .as_array()
        .expect("sequence must be array")
        .iter()
        .enumerate()
    {
        let input = step["input"].as_bool().expect("input is bool");
        let expected = step["expected"].as_bool().expect("expected is bool");
        let actual = filter.update(input);
        assert_eq!(
            actual, expected,
            "filter_debounce step {i} input={input}"
        );
    }
}

#[test]
fn observer_coolant_matches_reference() {
    let reference = load_reference();
    let spec = &reference["observers"]["observer_coolant"];
    let mut observer = ObserverCoolant::new();
    for (i, step) in spec["sequence"]
        .as_array()
        .expect("sequence must be array")
        .iter()
        .enumerate()
    {
        let input = step["input"].as_f64().expect("input is number");
        let expected_events: Vec<String> = step["expected_events"]
            .as_array()
            .expect("expected_events must be array")
            .iter()
            .map(|v| v.as_str().expect("event tag must be string").to_string())
            .collect();
        let queue = observer.update(input);
        let actual_events: Vec<String> = queue
            .as_slice()
            .iter()
            .map(|e| format!("{:?}", e.tag))
            .collect();
        assert_eq!(
            actual_events, expected_events,
            "observer_coolant step {i} input={input}"
        );
    }
}
