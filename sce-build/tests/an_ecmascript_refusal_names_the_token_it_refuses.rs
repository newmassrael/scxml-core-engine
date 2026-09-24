// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A statechart's refused ECMAScript names the token it refuses, where the
//! author wrote it.
//!
//! # What was wrong
//!
//! The acceptance walk anchored every refusal on the ELEMENT carrying the
//! expression and reported the payload's own reading as `actual`. Neither
//! is text the reported row must hold: a `cond` continued below its
//! `<transition` row was reported on that row, and a document that wrote
//! `words['map'](1)` was told `.map` — so SCE_ERROR_CONTRACT §3.1.1's
//! consumer searched a row for a token it does not hold, and a
//! `replace_one_of` had nothing to replace (measured 2026-09-24). The
//! frontend's tree carried no ranges, so the walk had nothing better to
//! report.
//!
//! # What is held
//!
//! Each case writes the refused token on a row its element does not start
//! on, or in a spelling the frontend normalises — a literal key, a space
//! inside a member access, an entity before it. Under every backend the
//! record must name that row and column and report the token as written,
//! read off the document rather than off this file; and a record proposing
//! a substitution must clear itself once the substitution is applied.
//!
//! This is the forge dialect's `a_lowering_refusal_names_the_token_it_refuses`
//! for the ECMAScript datamodel: one rule places both
//! (`sce_build::forge::expression_site::ExpressionSite::locate`).

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::forge::codegen_matrix::language_wire_name;
use sce_build::generator::Language;

fn codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

/// One document, and where in it the refused token is written.
struct Case {
    file: &'static str,
    document: &'static str,
    code: &'static str,
    line: u64,
    col: u64,
    /// The token as the document spells it on `line`.
    actual: &'static str,
}

/// A `cond` continued onto the rows below its `<transition`, the refused
/// method on the last of them.
const CONTINUED_COND: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" initial="a">
  <datamodel>
    <data id="count" expr="0"/>
    <data id="words" expr="['b','a']"/>
    <data id="n" expr="0"/>
  </datamodel>
  <state id="a">
    <transition event="go" target="b"
                cond="count &gt;= 0 &amp;&amp;
                      words.map(1).length"/>
  </state>
  <final id="b"/>
</scxml>
"#;

/// A method reached through a literal key, which the frontend folds into
/// the same node a dot makes.
const LITERAL_KEY: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" initial="a">
  <datamodel>
    <data id="count" expr="0"/>
    <data id="words" expr="['b','a']"/>
    <data id="n" expr="0"/>
  </datamodel>
  <state id="a">
    <onentry>
      <assign location="n"
              expr="words['map'](1).length"/>
    </onentry>
    <transition event="go" target="b"/>
  </state>
  <final id="b"/>
</scxml>
"#;

/// A member access with a space on either side of the dot.
const SPACED_MEMBER: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" initial="a">
  <datamodel>
    <data id="count" expr="0"/>
    <data id="words" expr="['b','a']"/>
    <data id="n" expr="0"/>
  </datamodel>
  <state id="a">
    <onentry>
      <assign location="n"
              expr="words . map(1)"/>
    </onentry>
    <transition event="go" target="b"/>
  </state>
  <final id="b"/>
</scxml>
"#;

/// An undeclared name after an operator the document escapes, so the
/// column counts the entity as written.
const AFTER_AN_ESCAPE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" initial="a">
  <datamodel>
    <data id="count" expr="0"/>
    <data id="words" expr="['b','a']"/>
    <data id="n" expr="0"/>
  </datamodel>
  <state id="a">
    <transition event="go" target="b"
                cond="1 &lt; conut"/>
  </state>
  <final id="b"/>
</scxml>
"#;

/// An `<elseif>`, whose condition is on its own element and not its
/// `<if>`'s.
const ELSEIF_ROW: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" initial="a">
  <datamodel>
    <data id="count" expr="0"/>
    <data id="words" expr="['b','a']"/>
    <data id="n" expr="0"/>
  </datamodel>
  <state id="a">
    <onentry>
      <if cond="count === 0">
        <log expr="'zero'"/>
      <elseif cond="Date.now() &gt; 0"/>
        <log expr="'later'"/>
      </if>
    </onentry>
    <transition event="go" target="b"/>
  </state>
  <final id="b"/>
</scxml>
"#;

/// A property called with nothing to pass, reached through a literal key:
/// the refused text is the key and the call together.
const LENGTH_CALL: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" initial="a">
  <datamodel>
    <data id="count" expr="0"/>
    <data id="words" expr="['b','a']"/>
    <data id="n" expr="0"/>
  </datamodel>
  <state id="a">
    <onentry>
      <assign location="n"
              expr="words['length']()"/>
    </onentry>
    <transition event="go" target="b"/>
  </state>
  <final id="b"/>
</scxml>
"#;

/// A `<send>` attribute on the second row of its start tag.
const SEND_SECOND_ROW: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" initial="a">
  <datamodel>
    <data id="count" expr="0"/>
    <data id="words" expr="['b','a']"/>
    <data id="n" expr="0"/>
  </datamodel>
  <state id="a">
    <onentry>
      <send target="#_internal"
            eventexpr="words.pop()"/>
    </onentry>
    <transition event="go" target="b"/>
  </state>
  <final id="b"/>
</scxml>
"##;

/// A namespace member reached through a literal key: the whole member is
/// what each candidate replaces.
const NAMESPACE_KEY: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" initial="a">
  <datamodel>
    <data id="count" expr="0"/>
    <data id="words" expr="['b','a']"/>
    <data id="n" expr="0"/>
  </datamodel>
  <state id="a">
    <onentry>
      <assign location="n"
              expr="JSON['serialize'](words)"/>
    </onentry>
    <transition event="go" target="b"/>
  </state>
  <final id="b"/>
</scxml>
"#;

/// A `<param>` value on the row below its element's start.
const PARAM_ROW: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" initial="a">
  <datamodel>
    <data id="count" expr="0"/>
    <data id="words" expr="['b','a']"/>
    <data id="n" expr="0"/>
  </datamodel>
  <state id="a">
    <onentry>
      <send event="e">
        <param name="p"
               expr="Math.tanh(1)"/>
      </send>
    </onentry>
    <transition event="go" target="b"/>
  </state>
  <final id="b"/>
</scxml>
"#;

/// A `<data>` value on the row below its element's start.
const DATA_ROW: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" initial="a">
  <datamodel>
    <data id="count" expr="0"/>
    <data id="words" expr="['b','a']"/>
    <data id="n" expr="0"/>
  </datamodel>
  <state id="a">
    <datamodel>
      <data id="tau"
            expr="Math.TAU"/>
    </datamodel>
    <transition event="go" target="b"/>
  </state>
  <final id="b"/>
</scxml>
"#;

/// An inline `<script>` body, the refused method on its second statement's
/// row — which is neither the `<script>` tag's row nor the body's first.
const INLINE_SCRIPT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" initial="a">
  <datamodel>
    <data id="count" expr="0"/>
    <data id="words" expr="['b','a']"/>
    <data id="n" expr="0"/>
  </datamodel>
  <state id="a">
    <onentry>
      <script>
        n = count + 1;
        n = words.map(1).length;
      </script>
    </onentry>
    <transition event="go" target="b"/>
  </state>
  <final id="b"/>
</scxml>
"#;

/// A top-level `<script>` whose body opens in a CDATA section and goes on
/// with entities, so the column counts the section's markers and each
/// entity as written.
const GLOBAL_SCRIPT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" initial="a">
  <datamodel>
    <data id="count" expr="0"/>
    <data id="words" expr="['b','a']"/>
    <data id="n" expr="0"/>
  </datamodel>
  <script><![CDATA[
n = count < 1;
]]>n = count &gt; 0 &amp;&amp; words.map(1).length;</script>
  <state id="a">
    <transition event="go" target="b"/>
  </state>
  <final id="b"/>
</scxml>
"#;

const CASES: &[Case] = &[
    Case {
        file: "inline_script.scxml",
        document: INLINE_SCRIPT,
        code: "expression/unsupported-builtin",
        line: 12,
        col: 18,
        actual: ".map",
    },
    Case {
        file: "global_script.scxml",
        document: GLOBAL_SCRIPT,
        code: "expression/unsupported-builtin",
        line: 10,
        col: 37,
        actual: ".map",
    },
    Case {
        file: "continued_cond.scxml",
        document: CONTINUED_COND,
        code: "expression/unsupported-builtin",
        line: 11,
        col: 28,
        actual: ".map",
    },
    Case {
        file: "literal_key.scxml",
        document: LITERAL_KEY,
        code: "expression/unsupported-builtin",
        line: 11,
        col: 26,
        actual: "['map']",
    },
    Case {
        file: "spaced_member.scxml",
        document: SPACED_MEMBER,
        code: "expression/unsupported-builtin",
        line: 11,
        col: 27,
        actual: ". map",
    },
    Case {
        file: "after_an_escape.scxml",
        document: AFTER_AN_ESCAPE,
        code: "expression/unknown-identifier",
        line: 10,
        col: 30,
        actual: "conut",
    },
    Case {
        file: "elseif_row.scxml",
        document: ELSEIF_ROW,
        code: "expression/unsupported-builtin",
        line: 12,
        col: 21,
        actual: "Date",
    },
    Case {
        file: "length_call.scxml",
        document: LENGTH_CALL,
        code: "expression/property-not-callable",
        line: 11,
        col: 26,
        actual: "['length']()",
    },
    Case {
        file: "send_second_row.scxml",
        document: SEND_SECOND_ROW,
        code: "expression/unsupported-builtin",
        line: 11,
        col: 29,
        actual: ".pop",
    },
    Case {
        file: "namespace_key.scxml",
        document: NAMESPACE_KEY,
        code: "expression/unsupported-builtin",
        line: 11,
        col: 21,
        actual: "JSON['serialize']",
    },
    Case {
        file: "param_row.scxml",
        document: PARAM_ROW,
        code: "expression/unsupported-builtin",
        line: 12,
        col: 22,
        actual: "Math.tanh",
    },
    Case {
        file: "data_row.scxml",
        document: DATA_ROW,
        code: "expression/unsupported-builtin",
        line: 11,
        col: 19,
        actual: "Math.TAU",
    },
];

/// Every case, written into one directory.
fn fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    for case in CASES {
        std::fs::write(dir.path().join(case.file), case.document).expect("write document");
    }
    dir
}

/// Every record `check` writes for `file` under `lang`. A refused ECMAScript
/// expression does not fail the build (W3C SCXML 5.9.1), so the run must
/// still succeed.
fn records(dir: &Path, file: &str, lang: Language) -> Result<Vec<serde_json::Value>, String> {
    let output = Command::new(codegen_bin())
        .current_dir(dir)
        .args([
            "--error-format=json",
            "check",
            file,
            "-l",
            language_wire_name(lang),
        ])
        .output()
        .expect("run sce-codegen");
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        return Err(format!("check failed: {stderr}"));
    }
    stderr
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('{'))
        .map(|line| serde_json::from_str(line).map_err(|e| format!("{e}: {line}")))
        .collect()
}

/// The one record of `case.code`, or why there is not exactly one.
fn refusal(dir: &Path, case: &Case, lang: Language) -> Result<serde_json::Value, String> {
    let all = records(dir, case.file, lang)?;
    let mut of_code = all.iter().filter(|r| r["code"] == case.code);
    match (of_code.next(), of_code.next()) {
        (Some(one), None) => Ok(one.clone()),
        _ => Err(format!("expected one {} record, got: {all:?}", case.code)),
    }
}

/// What is wrong with `record` as the refusal of `case`, if anything.
fn mismatches(case: &Case, record: &serde_json::Value) -> Vec<String> {
    let mut wrong = Vec::new();
    let expect = [
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
fn every_backend_names_the_row_and_column_of_the_refused_token() {
    let dir = fixture();
    let mut failures = Vec::new();
    for case in CASES {
        for &lang in Language::ALL {
            let wrong = match refusal(dir.path(), case, lang) {
                Ok(record) => mismatches(case, &record),
                Err(why) => vec![why],
            };
            if !wrong.is_empty() {
                failures.push(format!(
                    "{} [{}]: {}",
                    case.file,
                    language_wire_name(lang),
                    wrong.join("; ")
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} refusals are not placed at their token:\n{}",
        failures.len(),
        CASES.len() * Language::ALL.len(),
        failures.join("\n")
    );
}

/// A record proposing a substitution clears itself once the substitution
/// is applied where it says — the first candidate, on the row it names.
///
/// This is what the as-written `actual` is for: `.map` proposed against a
/// row that holds `['map']` replaced nothing, and the record stood.
#[test]
fn applying_the_proposed_substitution_clears_the_refusal() {
    let dir = fixture();
    let mut replayed = 0usize;
    let mut failures = Vec::new();
    for case in CASES {
        let record = match refusal(dir.path(), case, Language::Rust) {
            Ok(record) => record,
            Err(why) => {
                failures.push(format!("{}: {why}", case.file));
                continue;
            }
        };
        let replacement = match record["fix"]["kind"].as_str() {
            Some("replace_with") => record["fix"]["to"].as_str(),
            Some("replace_one_of") => record["fix"]["candidates"][0].as_str(),
            _ => None,
        };
        let Some(replacement) = replacement else {
            continue;
        };
        let repaired_dir = tempfile::tempdir().expect("tempdir");
        let repaired = common::wire_site::apply_substitution(
            case.document,
            record["actual"].as_str().expect("actual"),
            replacement,
            record["location"]["line"].as_u64().map(|n| n as usize),
        );
        std::fs::write(repaired_dir.path().join(case.file), repaired).expect("write repair");
        replayed += 1;
        match records(repaired_dir.path(), case.file, Language::Rust) {
            Ok(after) if after.iter().any(|r| r["id"] == record["id"]) => failures.push(format!(
                "{}: still refused once {:?} replaced {}",
                case.file, replacement, record["actual"]
            )),
            Ok(_) => {}
            Err(why) => failures.push(format!(
                "{}: the repair broke the document: {why}",
                case.file
            )),
        }
    }
    // Every case but none proposes a substitution; a replay that reached
    // none of them would pass by having done nothing.
    assert_eq!(
        replayed,
        CASES.len(),
        "cases without a substitution: {failures:?}"
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
