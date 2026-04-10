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
    pub mod transform_temperature {
        include!(concat!(env!("OUT_DIR"), "/transform_temperature.rs"));
    }
    pub mod transform_bitwise {
        include!(concat!(env!("OUT_DIR"), "/transform_bitwise.rs"));
    }
    pub mod transform_multi_output {
        include!(concat!(env!("OUT_DIR"), "/transform_multi_output.rs"));
    }
    pub mod condition_threshold {
        include!(concat!(env!("OUT_DIR"), "/condition_threshold.rs"));
    }
    pub mod condition_range {
        include!(concat!(env!("OUT_DIR"), "/condition_range.rs"));
    }
    pub mod condition_programming {
        include!(concat!(env!("OUT_DIR"), "/condition_programming.rs"));
    }
    pub mod procedure_linear {
        include!(concat!(env!("OUT_DIR"), "/procedure_linear.rs"));
    }
    pub mod procedure_diamond {
        include!(concat!(env!("OUT_DIR"), "/procedure_diamond.rs"));
    }
    pub mod procedure_startup_check {
        include!(concat!(env!("OUT_DIR"), "/procedure_startup_check.rs"));
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

#[test]
fn transform_temperature_matches_reference() {
    let reference = load_reference();
    let tol = tolerance(&reference);
    let spec = &reference["pure_functions"]["transform_temperature"];
    for case in spec["cases"].as_array().expect("cases must be array") {
        let raw = case["args"][0].as_u64().expect("raw is integer") as u16;
        let expected = case["expected"].as_f64().expect("expected is number");
        let actual = fixtures::transform_temperature::compute_temperature(raw);
        assert_close(
            actual,
            expected,
            tol,
            &format!("transform_temperature({raw})"),
        );
    }
}

#[test]
fn transform_bitwise_matches_reference() {
    let reference = load_reference();
    let spec = &reference["pure_functions"]["transform_bitwise"];
    for case in spec["cases"].as_array().expect("cases must be array") {
        let byte = case["args"][0].as_u64().expect("byte is integer") as u8;
        let expected_high = case["expected"]["high_nibble"]
            .as_u64()
            .expect("high_nibble is integer") as u8;
        let expected_low = case["expected"]["low_nibble"]
            .as_u64()
            .expect("low_nibble is integer") as u8;
        let actual_high = fixtures::transform_bitwise::compute_high_nibble(byte);
        let actual_low = fixtures::transform_bitwise::compute_low_nibble(byte);
        assert_eq!(
            actual_high, expected_high,
            "transform_bitwise({byte}) high"
        );
        assert_eq!(
            actual_low, expected_low,
            "transform_bitwise({byte}) low"
        );
    }
}

#[test]
fn transform_multi_output_matches_reference() {
    let reference = load_reference();
    let tol = tolerance(&reference);
    let spec = &reference["pure_functions"]["transform_multi_output"];
    for case in spec["cases"].as_array().expect("cases must be array") {
        let celsius = case["args"][0].as_f64().expect("celsius is number");
        let expected_f = case["expected"]["fahrenheit"]
            .as_f64()
            .expect("fahrenheit is number");
        let expected_k = case["expected"]["kelvin"]
            .as_f64()
            .expect("kelvin is number");
        let actual_f = fixtures::transform_multi_output::compute_fahrenheit(celsius);
        let actual_k = fixtures::transform_multi_output::compute_kelvin(celsius);
        assert_close(
            actual_f,
            expected_f,
            tol,
            &format!("transform_multi_output({celsius}) fahrenheit"),
        );
        assert_close(
            actual_k,
            expected_k,
            tol,
            &format!("transform_multi_output({celsius}) kelvin"),
        );
    }
}

#[test]
fn condition_threshold_matches_reference() {
    let reference = load_reference();
    let spec = &reference["pure_functions"]["condition_threshold"];
    for case in spec["cases"].as_array().expect("cases must be array") {
        let coolant = case["args"][0].as_f64().expect("coolant_temp is number");
        let oil = case["args"][1].as_f64().expect("oil_temp is number");
        let max = case["args"][2].as_f64().expect("max_temp is number");
        let expected = case["expected"].as_bool().expect("expected is bool");
        let actual = fixtures::condition_threshold::condition_threshold(coolant, oil, max);
        assert_eq!(
            actual, expected,
            "condition_threshold({coolant}, {oil}, {max})"
        );
    }
}

#[test]
fn condition_range_matches_reference() {
    let reference = load_reference();
    let spec = &reference["pure_functions"]["condition_range"];
    for case in spec["cases"].as_array().expect("cases must be array") {
        let rpm = case["args"][0].as_u64().expect("rpm is integer") as u32;
        let min_rpm = case["args"][1].as_u64().expect("min_rpm is integer") as u32;
        let max_rpm = case["args"][2].as_u64().expect("max_rpm is integer") as u32;
        let expected = case["expected"].as_bool().expect("expected is bool");
        let actual = fixtures::condition_range::condition_range(rpm, min_rpm, max_rpm);
        assert_eq!(
            actual, expected,
            "condition_range({rpm}, {min_rpm}, {max_rpm})"
        );
    }
}

#[test]
fn condition_programming_matches_reference() {
    let reference = load_reference();
    let spec = &reference["pure_functions"]["condition_programming"];
    for case in spec["cases"].as_array().expect("cases must be array") {
        let engine_stop = case["args"][0].as_bool().expect("engine_stop is bool");
        let ignition = case["args"][1].as_bool().expect("ignition is bool");
        let expected = case["expected"].as_bool().expect("expected is bool");
        let actual =
            fixtures::condition_programming::condition_programming(engine_stop, ignition);
        assert_eq!(
            actual, expected,
            "condition_programming({engine_stop}, {ignition})"
        );
    }
}

#[test]
fn procedure_linear_matches_reference() {
    let reference = load_reference();
    let spec = &reference["pure_functions"]["procedure_linear"];
    for case in spec["cases"].as_array().expect("cases must be array") {
        let value = case["args"][0].as_i64().expect("value is integer") as i32;
        let expected_completed = case["expected"]["completed"]
            .as_bool()
            .expect("completed is bool");
        let expected_final = case["expected"]["final_state"]
            .as_str()
            .expect("final_state is string");
        let result = fixtures::procedure_linear::execute(value);
        assert_eq!(
            result.completed, expected_completed,
            "procedure_linear({value}).completed"
        );
        assert_eq!(
            result.final_state.as_str(),
            expected_final,
            "procedure_linear({value}).final_state"
        );
    }
}

#[test]
fn procedure_diamond_matches_reference() {
    let reference = load_reference();
    let spec = &reference["pure_functions"]["procedure_diamond"];
    for case in spec["cases"].as_array().expect("cases must be array") {
        let sensor = case["args"][0].as_u64().expect("sensor_value is integer") as u16;
        let mode = case["args"][1].as_str().expect("mode is string");
        let expected_completed = case["expected"]["completed"]
            .as_bool()
            .expect("completed is bool");
        let expected_final = case["expected"]["final_state"]
            .as_str()
            .expect("final_state is string");
        let result = fixtures::procedure_diamond::execute(sensor, mode);
        assert_eq!(
            result.completed, expected_completed,
            "procedure_diamond({sensor}, {mode}).completed"
        );
        assert_eq!(
            result.final_state.as_str(),
            expected_final,
            "procedure_diamond({sensor}, {mode}).final_state"
        );
    }
}

#[test]
fn procedure_startup_check_matches_reference() {
    let reference = load_reference();
    let spec = &reference["pure_functions"]["procedure_startup_check"];
    for case in spec["cases"].as_array().expect("cases must be array") {
        let voltage = case["args"][0].as_f64().expect("voltage is number") as f32;
        let temperature = case["args"][1].as_f64().expect("temperature is number") as f32;
        let expected_completed = case["expected"]["completed"]
            .as_bool()
            .expect("completed is bool");
        let expected_final = case["expected"]["final_state"]
            .as_str()
            .expect("final_state is string");
        let result = fixtures::procedure_startup_check::execute(voltage, temperature);
        assert_eq!(
            result.completed, expected_completed,
            "procedure_startup_check({voltage}, {temperature}).completed"
        );
        assert_eq!(
            result.final_state.as_str(),
            expected_final,
            "procedure_startup_check({voltage}, {temperature}).final_state"
        );
    }
}
