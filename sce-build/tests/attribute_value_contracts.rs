// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The values an attribute may carry, held to the type of the field it
// annotates.
//
// Every case here generated with exit 0 before this file existed
// (measured 2026-09-21), and most produced code that no backend compiles:
// a validator bound that is not a number went into the check verbatim, an
// averaging filter took a `bool`, an interpolation table went to an `f64`
// runtime in `float32`, a unit annotated a string. The attributes are read
// by name, so nothing downstream ever asked what type they were about.
//
// Where the XSD already types an attribute (`sce:window` is a positive
// integer) the parser's own check is what an XSD-less build relies on,
// and it is covered in the parser's unit tests instead — through here the
// XSD answers first.

use sce_build::forge::error::{ForgeError, ValidationError};
use sce_build::generator::Language;

fn resource_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .join("tests/forge/resources")
}

fn compile(name: &str, scxml: &str) -> Result<String, ForgeError> {
    sce_build::compile_forge_with_imports(
        scxml,
        sce_build::DocumentLabel::symmetric(name),
        Language::Rust,
        &resource_dir(),
        &sce_build::ForgeCompileOptions::default(),
    )
    .map(|out| {
        out.files
            .iter()
            .map(|(_, content)| content.clone())
            .collect::<Vec<_>>()
            .join("\n")
    })
    .map_err(|located| located.error)
}

/// The validation error `scxml` is refused with.
fn refusal(name: &str, scxml: &str) -> ValidationError {
    match compile(name, scxml) {
        Ok(_) => panic!("{name}: generated, but must be refused"),
        Err(ForgeError::Validation(boxed)) => *boxed,
        Err(other) => panic!("{name}: refused, but not by validation: {other:?}"),
    }
}

fn assert_rule(name: &str, err: ValidationError, want_attr: &str) {
    match err {
        ValidationError::AttributeRuleViolated { attr, .. } => {
            assert_eq!(
                attr, want_attr,
                "{name}: the refusal names the wrong attribute"
            );
        }
        other => panic!("{name}: expected AttributeRuleViolated on {want_attr}, got {other:?}"),
    }
}

fn validator(name: &str, input: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="validator" name="{name}">
  <datamodel>
    {input}
    <data id="valid" sce:type="bool" sce:direction="out"/>
  </datamodel>
</scxml>"#
    )
}

#[test]
fn a_validator_bound_that_is_not_a_number_is_refused() {
    let doc = validator(
        "bound_word",
        r#"<data id="speed" sce:type="uint8" sce:direction="in" sce:range-min="low"/>"#,
    );
    assert_rule("bound_word", refusal("bound_word", &doc), "sce:range-min");
}

#[test]
fn a_validator_bound_outside_the_fields_type_is_refused() {
    let doc = validator(
        "bound_wide",
        r#"<data id="speed" sce:type="uint8" sce:direction="in" sce:range-max="300"/>"#,
    );
    assert_rule("bound_wide", refusal("bound_wide", &doc), "sce:range-max");
}

#[test]
fn a_validator_bound_on_a_bool_field_is_refused() {
    let doc = validator(
        "bound_bool",
        r#"<data id="armed" sce:type="bool" sce:direction="in" sce:range-min="0"/>"#,
    );
    assert_rule("bound_bool", refusal("bound_bool", &doc), "sce:range-min");
}

#[test]
fn a_validator_minimum_above_its_maximum_is_refused() {
    let doc = validator(
        "bound_crossed",
        r#"<data id="speed" sce:type="uint8" sce:direction="in"
                 sce:range-min="200" sce:range-max="100"/>"#,
    );
    assert_rule(
        "bound_crossed",
        refusal("bound_crossed", &doc),
        "sce:range-max",
    );
}

#[test]
fn a_validator_rate_limit_of_zero_is_refused() {
    let doc = validator(
        "roc_zero",
        r#"<data id="speed" sce:type="uint8" sce:direction="in" sce:max-delta="0"/>"#,
    );
    assert_rule("roc_zero", refusal("roc_zero", &doc), "sce:max-delta");
}

/// A bound is emitted in the spelling every backend reads the same way:
/// `0xFF` reaches the code as `255`, and `10` on a float as `10.0`.
#[test]
fn a_validator_bound_is_emitted_in_one_spelling() {
    let hex = validator(
        "bound_hex",
        r#"<data id="speed" sce:type="uint8" sce:direction="in" sce:range-min="0x10"/>"#,
    );
    let code = compile("bound_hex", &hex).expect("a hex bound on a uint8 generates");
    assert!(
        code.contains("speed < 16 "),
        "the bound must reach the check in decimal:\n{code}"
    );

    let float = validator(
        "bound_float",
        r#"<data id="temp" sce:type="float64" sce:direction="in" sce:range-max="10"/>"#,
    );
    let code = compile("bound_float", &float).expect("an integral bound on a float generates");
    assert!(
        code.contains("temp > 10.0 "),
        "a float bound must carry a decimal point:\n{code}"
    );
}

/// An unsigned lower bound of zero is elided, however it is spelt: `x < 0`
/// on an unsigned type is a tautology rustc refuses under
/// `-Dunused_comparisons`, and the template recognises zero by its text.
#[test]
fn a_zero_lower_bound_is_elided_in_any_spelling() {
    for zero in ["0", "0x0", "00", "0b0"] {
        let doc = validator(
            "bound_zero",
            &format!(
                r#"<data id="speed" sce:type="uint8" sce:direction="in"
                         sce:range-min="{zero}" sce:range-max="200"/>"#
            ),
        );
        let code = compile("bound_zero", &doc).expect("a zero lower bound generates");
        assert!(
            !code.contains("speed < "),
            "range-min=\"{zero}\" left a tautological comparison:\n{code}"
        );
        assert!(
            code.contains("speed > 200 "),
            "range-min=\"{zero}\" lost the upper check:\n{code}"
        );
    }
}

fn filter(name: &str, input_ty: &str, output: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="filter" name="{name}">
  <datamodel>
    <data id="raw" sce:type="{input_ty}" sce:direction="in"/>
    {output}
  </datamodel>
</scxml>"#
    )
}

#[test]
fn a_low_pass_weight_above_one_is_refused() {
    let doc = filter(
        "alpha_big",
        "float64",
        r#"<data id="out" sce:type="float64" sce:direction="out" sce:filter="low-pass" sce:alpha="2"/>"#,
    );
    assert_rule("alpha_big", refusal("alpha_big", &doc), "sce:alpha");
}

#[test]
fn an_averaging_filter_with_an_integer_output_is_refused() {
    let doc = filter(
        "ma_int_out",
        "uint16",
        r#"<data id="out" sce:type="uint16" sce:direction="out" sce:filter="moving-average" sce:window="4"/>"#,
    );
    match refusal("ma_int_out", &doc) {
        ValidationError::InvalidAttribute { attr, allowed, .. } => {
            assert_eq!(attr, "sce:type");
            assert_eq!(allowed, ["float32", "float64"]);
        }
        other => panic!("expected InvalidAttribute on the output type, got {other:?}"),
    }
}

#[test]
fn an_averaging_filter_over_a_bool_is_refused() {
    let doc = filter(
        "ma_bool_in",
        "bool",
        r#"<data id="out" sce:type="float64" sce:direction="out" sce:filter="moving-average" sce:window="4"/>"#,
    );
    assert_rule("ma_bool_in", refusal("ma_bool_in", &doc), "sce:type");
}

#[test]
fn a_debounce_passes_its_own_type_through() {
    let doc = filter(
        "debounce_retype",
        "bool",
        r#"<data id="out" sce:type="uint8" sce:direction="out" sce:filter="debounce" sce:window="3"/>"#,
    );
    match refusal("debounce_retype", &doc) {
        ValidationError::InvalidAttribute { attr, allowed, .. } => {
            assert_eq!(attr, "sce:type");
            assert_eq!(allowed, ["bool"]);
        }
        other => panic!("expected InvalidAttribute on the output type, got {other:?}"),
    }
}

fn interpolation(name: &str, body: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="interpolation" name="{name}">
  <datamodel>
    {body}
  </datamodel>
</scxml>"#
    )
}

/// An input declared below its output still has an axis. The axes used to
/// be read inside the loop, over the inputs seen so far.
#[test]
fn an_interpolation_input_declared_after_its_output_keeps_its_axis() {
    let doc = interpolation(
        "axis_after",
        r#"<data id="limit" sce:type="float64" sce:direction="out"
              sce:interpolation="linear" sce:axis-rpm="800 1200 2000">120.0 145.0 200.0</data>
    <data id="rpm" sce:type="uint16" sce:direction="in"/>"#,
    );
    compile("axis_after", &doc).expect("an input below its output keeps its axis");
}

#[test]
fn a_misspelt_axis_is_refused_with_the_declared_axes() {
    let doc = interpolation(
        "axis_typo",
        r#"<data id="rpm" sce:type="uint16" sce:direction="in"/>
    <data id="limit" sce:type="float64" sce:direction="out"
          sce:interpolation="linear" sce:axis-rmp="800 1200 2000">120.0 145.0 200.0</data>"#,
    );
    match refusal("axis_typo", &doc) {
        ValidationError::UnknownSceAttribute { attr, known, .. } => {
            assert_eq!(attr, "sce:axis-rmp");
            assert_eq!(known, ["sce:axis-rpm"]);
        }
        other => panic!("expected UnknownSceAttribute, got {other:?}"),
    }
}

#[test]
fn breakpoints_that_do_not_rise_are_refused() {
    let doc = interpolation(
        "axis_flat",
        r#"<data id="rpm" sce:type="uint16" sce:direction="in"/>
    <data id="limit" sce:type="float64" sce:direction="out"
          sce:interpolation="linear" sce:axis-rpm="800 800 2000">120.0 145.0 200.0</data>"#,
    );
    assert_rule("axis_flat", refusal("axis_flat", &doc), "sce:axis-rpm");
}

#[test]
fn an_interpolation_output_is_a_float64() {
    let doc = interpolation(
        "interp_f32",
        r#"<data id="rpm" sce:type="uint16" sce:direction="in"/>
    <data id="limit" sce:type="float32" sce:direction="out"
          sce:interpolation="linear" sce:axis-rpm="800 1200 2000">120.0 145.0 200.0</data>"#,
    );
    match refusal("interp_f32", &doc) {
        ValidationError::InvalidAttribute { attr, allowed, .. } => {
            assert_eq!(attr, "sce:type");
            assert_eq!(allowed, ["float64"]);
        }
        other => panic!("expected InvalidAttribute on the output type, got {other:?}"),
    }
}

#[test]
fn a_unit_on_a_non_numeric_field_is_refused() {
    let doc = validator(
        "unit_bool",
        r#"<data id="armed" sce:type="bool" sce:direction="in" sce:quantity="celsius"/>"#,
    );
    assert_rule("unit_bool", refusal("unit_bool", &doc), "sce:quantity");
}

/// An enum internal has no default to start from: zero need not be a
/// declared variant, and the generated Rust enum derives no `Default`.
#[test]
fn an_enum_internal_says_where_it_starts() {
    let doc = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="procedure" name="nrc_hold" initial="idle">
  <sce:import as="Nrc" src="enum_uds_nrc.scxml" kind="enum"/>
  <datamodel>
    <data id="value" sce:type="int32" sce:direction="in"/>
    <data id="last" sce:type="enum:Nrc" sce:direction="internal"/>
  </datamodel>
  <state id="idle">
    <transition target="done"/>
  </state>
  <final id="done"/>
</scxml>"#;
    match refusal("nrc_hold", doc) {
        ValidationError::MissingAttribute { element, attr } => {
            assert_eq!(element, "field 'last'");
            assert_eq!(attr, "expr");
        }
        other => panic!("expected MissingAttribute on expr, got {other:?}"),
    }
}

/// An expected value the return type cannot hold: `300` used to pass for a
/// `uint8` and be narrowed to 44 when the test was emitted.
#[test]
fn a_test_vector_value_outside_its_type_is_refused() {
    let doc = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" version="1.0" name="tv_wide">
  <sce:signature>
    <sce:param name="data" type="bytes"/>
    <sce:return type="uint8"/>
  </sce:signature>
  <sce:body>
    <sce:return expr="0"/>
  </sce:body>
  <sce:test-vector hex="00" value="300"/>
</scxml>"#;
    assert_rule("tv_wide", refusal("tv_wide", doc), "value");
}
