// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A refusal of an expression that holds a `cycle_*` call stands where the
//! refused text was written — in the host expression, or in a step's `when`.
//!
//! # What was wrong
//!
//! `cycle_expand` splices each `cycle_*` call into conditionals built from
//! the host expression and the cycle's `<sce:step when>` conditions, and the
//! expression checks run on the result. The field kept the spelling of what
//! its author wrote, which no longer spelled the expanded text, so the
//! placement rule placed the refusal nowhere: `conut` beside a
//! `cycle_next(…)`, and `normalOnn` in a step's `when`, were reported with a
//! file and no row (measured 2026-09-24).
//!
//! # What is held
//!
//! Each refusal names its token as written and stands on the row and column
//! that hold it — read off the fixture text, not restated — on every backend.

use std::process::Command;

use sce_build::forge::codegen_matrix::language_wire_name;
use sce_build::generator::Language;

const MODE_ENUM: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="enum" name="DriveMode" sce:underlying-type="uint8">
  <datamodel>
    <data id="variants">
      <sce:variant name="ECO" value="0"/>
      <sce:variant name="NORMAL" value="1"/>
      <sce:variant name="SPORT" value="2"/>
    </data>
  </datamodel>
</scxml>
"#;

/// A transform whose output navigates a cycle; `steps` and `expr` vary.
fn transform(steps: &str, expr: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="modes">
  <sce:import as="Mode" src="drivemode.scxml" kind="enum"/>
  <sce:cycle id="modes" of="Mode">
{steps}
  </sce:cycle>
  <datamodel>
    <data id="cursor" sce:type="enum:Mode" sce:direction="in"/>
    <data id="ecoOn" sce:type="bool" sce:direction="in"/>
    <data id="normalOn" sce:type="bool" sce:direction="in"/>
    <data id="count" sce:type="int32" sce:direction="in"/>
    <data id="nextOne" sce:type="int32" sce:direction="out"
          expr="{expr}"/>
  </datamodel>
</scxml>
"#
    )
}

const STEPS: &str = r#"    <sce:step name="ECO"    when="ecoOn"/>
    <sce:step name="NORMAL" when="normalOn"/>
    <sce:step name="SPORT"/>"#;

/// The row and column `needle` starts at in `text`, counted from 1, a
/// column per character.
fn written_at(text: &str, needle: &str) -> (u64, u64) {
    let offset = text.find(needle).expect("the fixture spells the needle");
    let before = &text[..offset];
    let row = before.matches('\n').count() as u64 + 1;
    let col = before[before.rfind('\n').map_or(0, |i| i + 1)..]
        .chars()
        .count() as u64
        + 1;
    (row, col)
}

/// The one `expression/unknown-identifier` record `check` writes for
/// `document` under `lang`.
fn refusal(document: &str, lang: Language) -> serde_json::Value {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("drivemode.scxml"), MODE_ENUM).expect("write enum");
    let path = dir.path().join("modes.scxml");
    std::fs::write(&path, document).expect("write document");
    let mut command = Command::new(env!("CARGO_BIN_EXE_sce-codegen"));
    command.args([
        "--error-format=json",
        "check",
        path.to_str().unwrap(),
        "-l",
        language_wire_name(lang),
    ]);
    // Go's imports are module-qualified; nothing to do with cycles.
    if lang == Language::Go {
        command.args(["--go-module-prefix", "example.com/generated"]);
    }
    let run = command.output().expect("spawn sce-codegen");
    let stderr = String::from_utf8_lossy(&run.stderr).into_owned();
    let records: Vec<serde_json::Value> = stderr
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('{'))
        .map(|line| serde_json::from_str(line).expect("one JSON record per line"))
        .collect();
    let mut unknown = records
        .iter()
        .filter(|r| r["code"] == "expression/unknown-identifier");
    match (unknown.next(), unknown.next()) {
        (Some(one), None) => one.clone(),
        _ => panic!("expected one unknown-identifier record, got:\n{stderr}"),
    }
}

/// `document`'s refusal of `token` stands where `token` is written, on
/// every backend.
fn assert_every_backend_places(document: &str, token: &str, needle: &str) {
    let (row, col) = written_at(document, needle);
    let col = col + needle.find(token).expect("the needle holds the token") as u64;
    let mut wrong = Vec::new();
    for &lang in Language::ALL {
        let record = refusal(document, lang);
        let placed = (
            record["location"]["line"].as_u64(),
            record["location"]["col"].as_u64(),
        );
        if record["actual"] != token || placed != (Some(row), Some(col)) {
            wrong.push(format!("{}: {record}", language_wire_name(lang)));
        }
    }
    assert!(
        wrong.is_empty(),
        "expected {token} at {row}:{col}:\n{}",
        wrong.join("\n")
    );
}

/// A name the author wrote beside a `cycle_*` call, in the host expression.
#[test]
fn a_host_token_beside_a_cycle_call_stands_where_it_was_written() {
    let document = transform(STEPS, "cycle_next(modes, cursor) + conut");
    assert_every_backend_places(&document, "conut", "+ conut");
}

/// A name the author wrote in a step's `when`, which reaches the checks only
/// spliced into the call's expansion.
#[test]
fn a_token_in_a_steps_when_stands_on_that_step() {
    let steps = STEPS.replace(r#"when="normalOn""#, r#"when="normalOnn""#);
    let document = transform(&steps, "cycle_next(modes, cursor)");
    assert_every_backend_places(&document, "normalOnn", "normalOnn\"");
}
