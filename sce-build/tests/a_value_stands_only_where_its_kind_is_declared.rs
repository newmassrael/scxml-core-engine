// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A forge value stands only where its kind is declared, and a call takes
//! the number of arguments its callee does.
//!
//! # What was wrong
//!
//! Extended SCXML is typed and admits no implicit coercion, and nothing
//! held a value to the type of the place it lands in. Measured 2026-09-24,
//! each of these generated with exit 0 on all six backends: a comparison
//! assigned to a `uint16` local (`x = reading == 3;`), a comparison as a
//! `float64` transform output, a real assigned to a `uint16` local, and an
//! imported two-parameter algorithm called with one argument, or with its
//! arguments' kinds swapped. C and C++ compiled the first two by converting
//! on their own, and Rust, Kotlin and Go refused them in their compilers —
//! the same document built or not by which backend was asked; rustc
//! refused the real as an `f32` assigned to a `u16`. The only place judged
//! was `<sce:append>`. An integer literal was held to nothing either: `300`
//! as a `uint8` generated everywhere, and rustc and `go build` refused it
//! while C, C++ and Kotlin made it 44 and Python kept 300.
//!
//! # What is held
//!
//! Each refusal is judged once, before any backend emits, so every backend
//! refuses the same document with the same record — placed at the value
//! (or, for a count, the callee) on a row its element does not start on.
//! The control documents are accepted everywhere: an integer stands as any
//! integer width or as a real, a literal as any type that holds it, and a
//! string stands as bytes.
//!
//! A callee's parameters are the ones its own document declares — an
//! algorithm's `<sce:signature>`, an interpolation's inputs, one per axis.
//! The interpolation import used to register an empty list for "not read",
//! which a count reads as "takes none": the rule refused the one correct
//! call and admitted no wrong one.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::forge::codegen_matrix::language_wire_name;
use sce_build::generator::Language;
use sce_build::ForgeCompileOptions;

fn codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

/// One document, and where in it the refused value is written.
struct Case {
    file: &'static str,
    document: &'static str,
    code: &'static str,
    line: u64,
    col: u64,
    /// The value as the document spells it on `line` — `None` when the
    /// record carries none (a count is no text of the document).
    actual: Option<&'static str>,
}

/// A comparison assigned to a `uint16` local.
const ASSIGN_BOOL: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_assign_bool" version="1.0">
  <sce:signature>
    <sce:param name="reading" type="uint16"/>
    <sce:return type="uint16"/>
  </sce:signature>
  <sce:body>
    <sce:var name="x" type="uint16" init="0"/>
    <sce:assign target="x"
                expr="reading === 3"/>
    <sce:return expr="x"/>
  </sce:body>
</scxml>
"#;

/// A real assigned to a `uint16` local — which rounding is meant is the
/// document's to say, with `round` or `floor`.
const ASSIGN_REAL: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_assign_real" version="1.0">
  <sce:signature>
    <sce:param name="reading" type="uint16"/>
    <sce:return type="uint16"/>
  </sce:signature>
  <sce:body>
    <sce:var name="x" type="uint16" init="0"/>
    <sce:assign target="x"
                expr="reading * 1.5"/>
    <sce:return expr="x"/>
  </sce:body>
</scxml>
"#;

/// A comparison as a `float64` output — its operator written as an entity,
/// which is how `actual` must spell it.
const OUTPUT_BOOL: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="transform" name="probe_output_bool" version="1.0">
  <datamodel>
    <data id="celsius" sce:type="float64" sce:direction="in"/>
    <data id="result" sce:type="float64" sce:direction="out"
          expr="celsius &gt; 1.0"/>
  </datamodel>
</scxml>
"#;

/// A string returned where the signature declares `uint16`.
const RETURN_STRING: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_return_string" version="1.0">
  <sce:signature>
    <sce:param name="reading" type="uint16"/>
    <sce:return type="uint16"/>
  </sce:signature>
  <sce:body>
    <sce:return
        expr="'high'"/>
  </sce:body>
</scxml>
"#;

/// A number as a condition's `bool` — C's truthiness, which a typed
/// language does not have.
const CONDITION_NUMBER: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="condition" name="probe_condition_number" version="1.0">
  <datamodel>
    <data id="flags" sce:type="uint8" sce:direction="in"/>
    <data id="result" sce:type="bool" sce:direction="out"
          expr="flags &amp; 1"/>
  </datamodel>
</scxml>
"#;

/// An imported two-parameter algorithm called with one argument.
const CALL_ARITY: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_call_arity" version="1.0">
  <sce:import kind="algorithm" src="probe_match.scxml" as="matched"/>
  <sce:signature>
    <sce:param name="reading" type="uint16"/>
    <sce:return type="uint16"/>
  </sce:signature>
  <sce:body>
    <sce:return
        expr="matched(reading)"/>
  </sce:body>
</scxml>
"#;

/// The same call with a number where its first parameter is a `bool`.
const CALL_ARGUMENT_KIND: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_call_argument_kind" version="1.0">
  <sce:import kind="algorithm" src="probe_match.scxml" as="matched"/>
  <sce:signature>
    <sce:param name="reading" type="uint16"/>
    <sce:return type="uint16"/>
  </sce:signature>
  <sce:body>
    <sce:return
        expr="matched(reading, reading)"/>
  </sce:body>
</scxml>
"#;

/// The statement form of that call, its argument on the tag's second row.
const STATEMENT_ARGUMENT_KIND: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_statement_argument_kind" version="1.0">
  <sce:import kind="algorithm" src="probe_match.scxml" as="matched"/>
  <sce:signature>
    <sce:param name="reading" type="uint16"/>
    <sce:return type="uint16"/>
  </sce:signature>
  <sce:body>
    <sce:call target="matched"
              args="reading, 1"/>
    <sce:return expr="reading"/>
  </sce:body>
</scxml>
"#;

/// An imported one-axis interpolation called with two arguments.
const INTERPOLATION_ARITY: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="validator" version="1.0">
  <sce:import src="probe_limit.scxml" kind="interpolation" as="limit"/>
  <datamodel>
    <data id="rpm" sce:type="uint16" sce:direction="in"/>
    <data id="valid" sce:type="bool" sce:direction="out"
          sce:plausibility="limit(rpm, rpm) &gt; 200.0"/>
  </datamodel>
</scxml>
"#;

/// A stateful import's method called with two arguments where it takes one.
/// C11 lowers such a call to its own free function, and until 2026-09-24
/// did so before any check ran, so C11 alone generated this and the next.
const METHOD_ARITY: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="procedure" initial="sample" version="1.0">
  <sce:import src="probe_smoother.scxml" kind="filter" as="smoother"/>
  <datamodel>
    <data id="rawSample" sce:type="float64" sce:direction="in"/>
    <data id="smoothed" sce:type="float64" sce:direction="internal"/>
  </datamodel>
  <state id="sample">
    <transition target="done">
      <assign location="smoothed"
              expr="smoother.update(rawSample, rawSample)"/>
    </transition>
  </state>
  <final id="done">
    <donedata><param name="result" expr="'success'"/></donedata>
  </final>
</scxml>
"#;

/// The same method's `float64` result assigned to a `bool` local.
const METHOD_SLOT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="procedure" initial="sample" version="1.0">
  <sce:import src="probe_smoother.scxml" kind="filter" as="smoother"/>
  <datamodel>
    <data id="rawSample" sce:type="float64" sce:direction="in"/>
    <data id="flag" sce:type="bool" sce:direction="internal"/>
  </datamodel>
  <state id="sample">
    <transition target="done">
      <assign location="flag"
              expr="smoother.update(rawSample)"/>
    </transition>
  </state>
  <final id="done">
    <donedata><param name="result" expr="'success'"/></donedata>
  </final>
</scxml>
"#;

/// The filter the method cases import: `update` takes one `float64`.
const PROBE_SMOOTHER: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="filter" version="1.0">
  <datamodel>
    <data id="rawSignal" sce:type="float64" sce:direction="in"/>
    <data id="smoothed" sce:type="float64" sce:direction="out"
          sce:filter="low-pass" sce:alpha="0.1"/>
  </datamodel>
</scxml>
"#;

/// The algorithm the call cases import: a `bool` then a `uint16`.
const PROBE_MATCH: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_match" version="1.0">
  <sce:signature>
    <sce:param name="same" type="bool"/>
    <sce:param name="level" type="uint16"/>
    <sce:return type="uint16"/>
  </sce:signature>
  <sce:body>
    <sce:return expr="level"/>
  </sce:body>
</scxml>
"#;

/// Everything the rule must still accept: an integer where a wider or a
/// narrower integer is declared, or a real; a real made whole by `round`;
/// literals at the very edges of their types; a string literal read as
/// bytes; and a call whose arguments are the kinds its parameters declare.
const CONTROL: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_control" version="1.0">
  <sce:import kind="algorithm" src="probe_match.scxml" as="matched"/>
  <sce:signature>
    <sce:param name="reading" type="uint16"/>
    <sce:param name="frame" type="bytes"/>
    <sce:return type="uint16"/>
  </sce:signature>
  <sce:body>
    <sce:var name="wide" type="uint32" init="reading"/>
    <sce:var name="low" type="uint8" init="reading"/>
    <sce:var name="ratio" type="float64" init="reading"/>
    <sce:var name="scaled" type="uint16" init="round(ratio * 1.5)"/>
    <sce:var name="edge" type="int8" init="-128"/>
    <sce:var name="full" type="uint8" init="0xFF"/>
    <sce:var name="top" type="uint8" init="255"/>
    <sce:var name="same" type="bool" init="frame === 'ab'"/>
    <sce:return expr="matched(same, reading)"/>
  </sce:body>
</scxml>
"#;

/// `300` as a `uint8` local's initial value.
const LITERAL_LOCAL: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_literal_local" version="1.0">
  <sce:signature>
    <sce:param name="reading" type="uint16"/>
    <sce:return type="uint8"/>
  </sce:signature>
  <sce:body>
    <sce:var name="n" type="uint8"
             init="300"/>
    <sce:return expr="n"/>
  </sce:body>
</scxml>
"#;

/// `300` meeting a `uint8` operand: the literal takes its partner's type,
/// whatever the result flows into.
const LITERAL_OPERAND: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_literal_operand" version="1.0">
  <sce:signature>
    <sce:param name="reading" type="uint8"/>
    <sce:return type="uint16"/>
  </sce:signature>
  <sce:body>
    <sce:return
        expr="reading + 300"/>
  </sce:body>
</scxml>
"#;

/// `-1` passed to a `uint16` parameter — the minus is part of the literal.
const LITERAL_NEGATIVE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_literal_negative" version="1.0">
  <sce:import kind="algorithm" src="probe_match.scxml" as="matched"/>
  <sce:signature>
    <sce:param name="reading" type="uint16"/>
    <sce:return type="uint16"/>
  </sce:signature>
  <sce:body>
    <sce:return
        expr="matched(true, -1)"/>
  </sce:body>
</scxml>
"#;

/// The interpolation the interpolation cases import: one axis, `rpm`.
const PROBE_LIMIT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="interpolation" version="1.0">
  <datamodel>
    <data id="rpm" sce:type="uint16" sce:direction="in"/>
    <data id="limit" sce:type="float64" sce:direction="out"
          sce:interpolation="linear" sce:out-of-bounds="clamp"
          sce:axis-rpm="800 1200 2000">
      120.0 145.0 200.0
    </data>
  </datamodel>
</scxml>
"#;

/// The same interpolation called with the one argument its axis takes.
const INTERPOLATION_CONTROL: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="validator" version="1.0">
  <sce:import src="probe_limit.scxml" kind="interpolation" as="limit"/>
  <datamodel>
    <data id="rpm" sce:type="uint16" sce:direction="in"/>
    <data id="valid" sce:type="bool" sce:direction="out"
          sce:plausibility="limit(rpm) &gt; 200.0"/>
  </datamodel>
</scxml>
"#;

/// Documents the rule must accept on every backend.
const CONTROLS: &[&str] = &["probe_control.scxml", "probe_interpolation_control.scxml"];

/// Documents the cases import or the controls stand on, written beside them.
const SUPPORT: &[(&str, &str)] = &[
    ("probe_match.scxml", PROBE_MATCH),
    ("probe_limit.scxml", PROBE_LIMIT),
    ("probe_smoother.scxml", PROBE_SMOOTHER),
    ("probe_control.scxml", CONTROL),
    ("probe_interpolation_control.scxml", INTERPOLATION_CONTROL),
];

const CASES: &[Case] = &[
    Case {
        file: "probe_assign_bool.scxml",
        document: ASSIGN_BOOL,
        code: "expression/type-mismatch",
        line: 10,
        col: 23,
        actual: Some("reading === 3"),
    },
    Case {
        file: "probe_assign_real.scxml",
        document: ASSIGN_REAL,
        code: "expression/type-mismatch",
        line: 10,
        col: 23,
        actual: Some("reading * 1.5"),
    },
    Case {
        file: "probe_output_bool.scxml",
        document: OUTPUT_BOOL,
        code: "expression/type-mismatch",
        line: 6,
        col: 17,
        actual: Some("celsius &gt; 1.0"),
    },
    Case {
        file: "probe_return_string.scxml",
        document: RETURN_STRING,
        code: "expression/type-mismatch",
        line: 9,
        col: 15,
        actual: Some("'high'"),
    },
    Case {
        file: "probe_condition_number.scxml",
        document: CONDITION_NUMBER,
        code: "expression/type-mismatch",
        line: 6,
        col: 17,
        actual: Some("flags &amp; 1"),
    },
    Case {
        file: "probe_call_arity.scxml",
        document: CALL_ARITY,
        code: "expression/argument-count-mismatch",
        line: 10,
        col: 15,
        actual: None,
    },
    Case {
        file: "probe_call_argument_kind.scxml",
        document: CALL_ARGUMENT_KIND,
        code: "expression/type-mismatch",
        line: 10,
        col: 23,
        actual: Some("reading"),
    },
    Case {
        file: "probe_statement_argument_kind.scxml",
        document: STATEMENT_ARGUMENT_KIND,
        code: "expression/type-mismatch",
        line: 10,
        col: 21,
        actual: Some("reading"),
    },
    Case {
        file: "probe_interpolation_arity.scxml",
        document: INTERPOLATION_ARITY,
        code: "expression/argument-count-mismatch",
        line: 7,
        col: 29,
        actual: None,
    },
    Case {
        file: "probe_method_arity.scxml",
        document: METHOD_ARITY,
        code: "expression/argument-count-mismatch",
        line: 11,
        col: 21,
        actual: None,
    },
    Case {
        file: "probe_method_slot.scxml",
        document: METHOD_SLOT,
        code: "expression/type-mismatch",
        line: 11,
        col: 21,
        actual: Some("smoother.update(rawSample)"),
    },
    Case {
        file: "probe_literal_local.scxml",
        document: LITERAL_LOCAL,
        code: "expression/literal-out-of-range",
        line: 9,
        col: 20,
        actual: Some("300"),
    },
    Case {
        file: "probe_literal_operand.scxml",
        document: LITERAL_OPERAND,
        code: "expression/literal-out-of-range",
        line: 9,
        col: 25,
        actual: Some("300"),
    },
    Case {
        file: "probe_literal_negative.scxml",
        document: LITERAL_NEGATIVE,
        code: "expression/literal-out-of-range",
        line: 10,
        col: 29,
        actual: Some("-1"),
    },
];

/// Every case and every support document, written into one directory.
fn fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let cases = CASES.iter().map(|case| (case.file, case.document));
    for (file, document) in cases.chain(SUPPORT.iter().copied()) {
        std::fs::write(dir.path().join(file), document).expect("write document");
    }
    dir
}

/// `sce-codegen check <file> -l <wire>` in JSON mode: its exit status and
/// its non-empty stderr lines.
fn check(dir: &Path, file: &str, wire: &str) -> (bool, Vec<String>) {
    let output = Command::new(codegen_bin())
        .current_dir(dir)
        // Go refuses any `<sce:import>` without the module path its imports
        // are qualified by; the other backends ignore it.
        .args([
            "--error-format=json",
            "check",
            file,
            "-l",
            wire,
            "--go-module-prefix",
            "example.com/probe",
        ])
        .output()
        .expect("run sce-codegen");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let lines = stderr
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(str::to_string)
        .collect();
    (output.status.success(), lines)
}

/// What is wrong with `record` as the refusal of `case`, if anything.
fn mismatches(case: &Case, record: &serde_json::Value) -> Vec<String> {
    let mut wrong = Vec::new();
    let expect = [
        ("code", record["code"].clone(), serde_json::json!(case.code)),
        (
            "location.line",
            record["location"]["line"].clone(),
            serde_json::json!(case.line),
        ),
        (
            "location.col",
            record["location"]["col"].clone(),
            serde_json::json!(case.col),
        ),
        (
            "actual",
            record["actual"].clone(),
            serde_json::json!(case.actual),
        ),
    ];
    for (field, got, want) in expect {
        if got != want {
            wrong.push(format!("{field} = {got}, want {want}"));
        }
    }
    // §3.1.1, read off the document rather than off the fixture's claim.
    if let Some(why) = common::wire_site::record_misplaced(record, case.document) {
        wrong.push(format!(
            "actual {} cannot be found: {why}",
            record["actual"]
        ));
    }
    wrong
}

#[test]
fn every_backend_refuses_the_value_where_it_is_written() {
    let dir = fixture();
    let mut failures = Vec::new();
    for case in CASES {
        for &lang in Language::ALL {
            let wire = language_wire_name(lang);
            let wrong = match check(dir.path(), case.file, wire) {
                (true, lines) => vec![format!("accepted: {lines:?}")],
                (false, lines) => match lines.as_slice() {
                    [one] => match serde_json::from_str(one) {
                        Ok(record) => mismatches(case, &record),
                        Err(e) => vec![format!("{e}: {one}")],
                    },
                    _ => vec![format!("expected one record: {lines:?}")],
                },
            };
            if !wrong.is_empty() {
                failures.push(format!("{} [{wire}]: {}", case.file, wrong.join("; ")));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} refusals are wrong:\n{}",
        failures.len(),
        CASES.len() * Language::ALL.len(),
        failures.join("\n")
    );
}

/// The rule refuses kinds, not conversions: every control document is
/// accepted on every backend.
#[test]
fn every_backend_accepts_what_the_rule_admits() {
    let dir = fixture();
    let mut refused = Vec::new();
    for &file in CONTROLS {
        for &lang in Language::ALL {
            let wire = language_wire_name(lang);
            if let (false, lines) = check(dir.path(), file, wire) {
                refused.push(format!("{file} [{wire}] {lines:?}"));
            }
        }
    }
    assert!(refused.is_empty(), "{}", refused.join("\n"));
}

/// The library facade places each refusal as the CLI does.
#[test]
fn the_library_route_places_each_refusal_as_the_cli_does() {
    let dir = fixture();
    let mut failures = Vec::new();
    for case in CASES {
        match sce_build::compile_forge_file(
            &dir.path().join(case.file),
            Language::Rust,
            &[],
            &ForgeCompileOptions::default(),
        ) {
            Ok(_) => failures.push(format!("{}: accepted", case.file)),
            Err(err) => {
                let at = (err.location.line, err.location.col);
                let want = (Some(case.line as u32), Some(case.col as u32));
                if at != want {
                    failures.push(format!("{}: at {at:?}, want {want:?}: {err}", case.file));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
