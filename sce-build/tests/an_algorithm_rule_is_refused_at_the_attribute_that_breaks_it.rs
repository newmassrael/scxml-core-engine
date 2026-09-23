// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A rule an algorithm document breaks is refused where the author broke
//! it: at the attribute, on the row and column it is written at.
//!
//! # What was wrong
//!
//! The algorithm kind judges some rules only once its body is read — a
//! parameter is read-only, a `bytes` buffer and its `capacity` agree with
//! the signature — and those refusals were placed at `<sce:body>`, the one
//! element the check still held: "the nearest container element the
//! diagnostic can point to without re-threading nodes through the IR".
//! Measured 2026-09-23, every one of them named a row that did not hold its
//! token, so `SCE_ERROR_CONTRACT.md` §3.1.1 — `actual` occurs on the line
//! the record names — failed for each, and a scalar's stray `capacity` was
//! reported as the text `(present)`, which no document contains.
//!
//! Four integer attributes (`capacity`, `max-iter`, `returns-max-size`,
//! `max-count`) were read so that a value outside `u32` became ABSENT. The
//! schema's integer types are unbounded, so `4294967296` passed it and then
//! vanished: a `max-iter` bound was dropped, a buffer's `capacity` was
//! reported missing. And `returns-max-size` on a return that is not `bytes`
//! was accepted and dropped, on the algorithm's signature and on a
//! procedure's `<sce:helper>` alike.
//!
//! # What is held
//!
//! Each case writes the attribute on a row its element does not start on,
//! so a record placed at the element — or at `<sce:body>` — is caught.
//! `actual` must also occur on the row the record names, read off the
//! document rather than off the fixture's claim.

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::generator::Language;
use sce_build::ForgeCompileOptions;

fn codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

/// One document, and where in it the refused attribute is written.
struct Case {
    file: &'static str,
    document: &'static str,
    code: &'static str,
    line: u64,
    col: u64,
    /// The value as the document spells it on `line` — `None` when the
    /// refusal is of something not written at all (a missing attribute).
    actual: Option<&'static str>,
}

/// A parameter assigned inside a nested block, its name on the assign's
/// second row.
const LVALUE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_lvalue" version="1.0">
  <sce:signature>
    <sce:param name="data" type="uint16"/>
    <sce:return type="uint16"/>
  </sce:signature>
  <sce:body>
    <sce:var name="x" type="uint16" init="0"/>
    <sce:if cond="x === 0">
      <sce:assign expr="1"
                  target="data"/>
    </sce:if>
    <sce:return expr="x"/>
  </sce:body>
</scxml>
"#;

/// A scalar local with a `capacity`, which only a `bytes` buffer takes.
const SCALAR_CAPACITY: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_scalar_capacity" version="1.0">
  <sce:signature>
    <sce:param name="data" type="uint16"/>
    <sce:return type="uint16"/>
  </sce:signature>
  <sce:body>
    <sce:var name="x" type="uint16" init="0"
             capacity="4"/>
    <sce:return expr="x"/>
  </sce:body>
</scxml>
"#;

/// A `bytes` return with no `returns-max-size` — refused at the
/// `<sce:return>`, which is where the attribute belongs.
const RETURN_WITHOUT_CAP: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_return_without_cap" version="1.0">
  <sce:signature>
    <sce:param name="data" type="uint16"/>
    <sce:return
        type="bytes"/>
  </sce:signature>
  <sce:body>
    <sce:var name="out" type="bytes" capacity="4"/>
    <sce:return expr="out"/>
  </sce:body>
</scxml>
"#;

/// A `returns-max-size` on a scalar return, which has no buffer to bound.
const SCALAR_RETURN_CAP: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_scalar_return_cap" version="1.0">
  <sce:signature>
    <sce:param name="data" type="uint16"/>
    <sce:return type="uint16"
                returns-max-size="8"/>
  </sce:signature>
  <sce:body>
    <sce:return expr="data"/>
  </sce:body>
</scxml>
"#;

/// A `bytes` buffer with no `capacity` — refused at its `<sce:var>`.
const BUFFER_WITHOUT_CAP: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_buffer_without_cap" version="1.0">
  <sce:signature>
    <sce:param name="data" type="uint16"/>
    <sce:return type="bytes" returns-max-size="4"/>
  </sce:signature>
  <sce:body>
    <sce:var name="out"
             type="bytes"/>
    <sce:return expr="out"/>
  </sce:body>
</scxml>
"#;

/// A buffer whose `capacity` is not the signature's `returns-max-size`.
const CAPACITY_MISMATCH: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_capacity_mismatch" version="1.0">
  <sce:signature>
    <sce:param name="data" type="uint16"/>
    <sce:return type="bytes" returns-max-size="16"/>
  </sce:signature>
  <sce:body>
    <sce:var name="out" type="bytes"
             capacity="8"/>
    <sce:return expr="out"/>
  </sce:body>
</scxml>
"#;

/// A `capacity` one past `u32::MAX`, which the unbounded schema type admits.
const CAPACITY_OVERFLOW: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_capacity_overflow" version="1.0">
  <sce:signature>
    <sce:param name="data" type="uint16"/>
    <sce:return type="bytes" returns-max-size="4"/>
  </sce:signature>
  <sce:body>
    <sce:var name="out" type="bytes"
             capacity="4294967296"/>
    <sce:return expr="out"/>
  </sce:body>
</scxml>
"#;

/// A `max-iter` one past `u32::MAX` — the bound that used to vanish.
const MAX_ITER_OVERFLOW: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_max_iter_overflow" version="1.0">
  <sce:signature>
    <sce:param name="data" type="uint16"/>
    <sce:return type="uint16"/>
  </sce:signature>
  <sce:body>
    <sce:var name="x" type="uint16" init="0"/>
    <sce:while cond="x &lt; 3"
               max-iter="4294967296">
      <sce:assign target="x" expr="x + 1"/>
    </sce:while>
    <sce:return expr="x"/>
  </sce:body>
</scxml>
"#;

/// A procedure helper returning a scalar, with a `returns-max-size`.
const HELPER_CAP: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="procedure" name="probe_helper_cap" initial="start">
  <datamodel>
    <data id="value" sce:type="uint8" sce:direction="in"/>
    <sce:helper name="scale" args="uint8" returns="uint16"
                sce:returns-max-size="8"/>
  </datamodel>
  <state id="start">
    <transition event="go" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#;

/// A second `bytes` buffer, where v1 takes one — refused at the second.
const EXTRA_BUFFER: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_extra_buffer" version="1.0">
  <sce:signature>
    <sce:param name="data" type="uint16"/>
    <sce:return type="bytes" returns-max-size="4"/>
  </sce:signature>
  <sce:body>
    <sce:var name="out" type="bytes" capacity="4"/>
    <sce:var type="bytes" capacity="4"
             name="spare"/>
    <sce:return expr="out"/>
  </sce:body>
</scxml>
"#;

/// A `bytes` buffer in an algorithm that does not return `bytes`.
const BUFFER_NOT_RETURNED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_buffer_not_returned" version="1.0">
  <sce:signature>
    <sce:param name="data" type="uint16"/>
    <sce:return type="uint16"/>
  </sce:signature>
  <sce:body>
    <sce:var type="bytes" capacity="4"
             name="out"/>
    <sce:return expr="data"/>
  </sce:body>
</scxml>
"#;

const CASES: &[Case] = &[
    Case {
        file: "probe_lvalue.scxml",
        document: LVALUE,
        code: "algorithm/lvalue-unsupported",
        line: 11,
        col: 27,
        actual: Some("data"),
    },
    Case {
        file: "probe_scalar_capacity.scxml",
        document: SCALAR_CAPACITY,
        code: "validation/attribute-rule-violated",
        line: 9,
        col: 14,
        actual: Some("4"),
    },
    Case {
        file: "probe_return_without_cap.scxml",
        document: RETURN_WITHOUT_CAP,
        code: "validation/missing-attribute",
        line: 5,
        col: 5,
        actual: None,
    },
    Case {
        file: "probe_scalar_return_cap.scxml",
        document: SCALAR_RETURN_CAP,
        code: "validation/attribute-rule-violated",
        line: 6,
        col: 17,
        actual: Some("8"),
    },
    Case {
        file: "probe_buffer_without_cap.scxml",
        document: BUFFER_WITHOUT_CAP,
        code: "validation/missing-attribute",
        line: 8,
        col: 5,
        actual: None,
    },
    Case {
        file: "probe_capacity_mismatch.scxml",
        document: CAPACITY_MISMATCH,
        code: "validation/invalid-attribute",
        line: 9,
        col: 24,
        actual: Some("8"),
    },
    Case {
        file: "probe_capacity_overflow.scxml",
        document: CAPACITY_OVERFLOW,
        code: "validation/attribute-rule-violated",
        line: 9,
        col: 14,
        actual: Some("4294967296"),
    },
    Case {
        file: "probe_max_iter_overflow.scxml",
        document: MAX_ITER_OVERFLOW,
        code: "validation/attribute-rule-violated",
        line: 10,
        col: 16,
        actual: Some("4294967296"),
    },
    Case {
        file: "probe_helper_cap.scxml",
        document: HELPER_CAP,
        code: "validation/attribute-rule-violated",
        line: 6,
        col: 17,
        actual: Some("8"),
    },
    Case {
        file: "probe_extra_buffer.scxml",
        document: EXTRA_BUFFER,
        code: "validation/incompatible-attributes",
        line: 10,
        col: 20,
        actual: None,
    },
    Case {
        file: "probe_buffer_not_returned.scxml",
        document: BUFFER_NOT_RETURNED,
        code: "validation/incompatible-attributes",
        line: 9,
        col: 20,
        actual: None,
    },
];

/// Every case, written into one directory.
fn fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    for case in CASES {
        std::fs::write(dir.path().join(case.file), case.document).expect("write case");
    }
    dir
}

/// The one record `sce-codegen check` prints for `file` in JSON mode, or why
/// there is not exactly one. The rules are judged before any backend is
/// chosen, so one backend asks for all of them.
fn refusal(dir: &Path, file: &str) -> Result<serde_json::Value, String> {
    let output = Command::new(codegen_bin())
        .current_dir(dir)
        .args(["--error-format=json", "check", file, "-l", "rust"])
        .output()
        .expect("run sce-codegen");
    let stderr = String::from_utf8_lossy(&output.stderr);
    if output.status.success() {
        return Err(format!("accepted the document; stderr: {stderr}"));
    }
    let lines: Vec<&str> = stderr.lines().filter(|l| !l.trim().is_empty()).collect();
    match lines.as_slice() {
        [one] => serde_json::from_str(one).map_err(|e| format!("{e}: {stderr}")),
        _ => Err(format!("expected one record:\n{stderr}")),
    }
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
    if let (Some(line), Some(actual)) = (
        record["location"]["line"].as_u64(),
        record["actual"].as_str(),
    ) {
        let row = case.document.lines().nth(line as usize - 1).unwrap_or("");
        if !row.contains(actual) {
            wrong.push(format!(
                "actual {actual:?} does not occur on line {line}: {row:?}"
            ));
        }
    }
    wrong
}

#[test]
fn each_rule_names_the_row_and_column_of_its_attribute() {
    let dir = fixture();
    let mut failures = Vec::new();
    for case in CASES {
        let wrong = match refusal(dir.path(), case.file) {
            Ok(record) => mismatches(case, &record),
            Err(why) => vec![why],
        };
        if !wrong.is_empty() {
            failures.push(format!("{}: {}", case.file, wrong.join("; ")));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} refusals are not placed at their attribute:\n{}",
        failures.len(),
        CASES.len(),
        failures.join("\n")
    );
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
