// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A refusal raised while an expression is lowered names the token it
//! refuses, where the author wrote it.
//!
//! # What was wrong
//!
//! A forge expression is judged in full only when it is lowered: the name
//! checks, the grammar and the per-backend emitters all run inside code
//! generation, after the document has become a model. Those refusals left
//! the generator with a file and nothing else — `expression/unknown-identifier`
//! for `celsius + conut` reached the wire as `location: {file}` (measured
//! 2026-09-23), though the model had kept every one of these attributes as
//! written, row and column included, for exactly this purpose
//! ([`sce_build::attribute_spelling`]).
//!
//! Without a row the §3.1.1 contract falls back to searching the whole
//! document for `actual`, which fails the moment the token occurs twice —
//! and a misspelled name is usually a near miss of one written elsewhere.
//!
//! # What is held
//!
//! Each case writes the refused token at a row and column the fixture
//! states, on a row that is NOT the row its element starts on, so a record
//! placed at the element instead of the token is caught. Every backend is
//! asked, because each lowers the expression for itself: a backend that
//! forgot to place its refusal would pass a single-language test.
//!
//! `actual` must also be findable on the row it names, exactly as the
//! document spells it — `&lt;` rather than the `<` a reader decoded it to —
//! which is the property `diagnostic_fix_is_applicable.rs` enforces over the
//! tracked corpus, asserted here for documents that corpus does not hold.

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::forge::codegen_matrix::language_wire_name;
use sce_build::generator::Language;
use sce_build::ForgeCompileOptions;

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
    /// The token as the document spells it on `line` — `None` when what is
    /// refused is written nowhere.
    actual: Option<&'static str>,
}

/// A transform output continued onto a second row, the undeclared name on
/// the continuation — neither the element's row nor the attribute's.
const TRANSFORM_CONTINUED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="transform" name="probe_transform">
  <datamodel>
    <data id="celsius" sce:type="float64" sce:direction="in"/>
    <data id="result" sce:type="float64" sce:direction="out"
          expr="celsius * 1.8
                + conut"/>
  </datamodel>
</scxml>
"#;

/// A condition whose name follows an operator the document escapes, so the
/// column counts the escape as written.
const CONDITION_AFTER_AN_ESCAPE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="condition" name="probe_condition">
  <datamodel>
    <data id="ignition" sce:type="bool" sce:direction="in"/>
    <data id="result" sce:type="bool" sce:direction="out"
          expr="ignition === true &amp;&amp; conut"/>
  </datamodel>
</scxml>
"#;

/// A validator's plausibility rule.
const VALIDATOR_PLAUSIBILITY: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="validator" name="probe_validator">
  <datamodel>
    <data id="temperature" sce:type="float64" sce:direction="in"
          sce:range-min="-40.0" sce:range-max="150.0"/>
    <data id="valid" sce:type="bool" sce:direction="out"
          sce:plausibility="temperature &gt; conut"/>
  </datamodel>
</scxml>
"#;

/// A procedure internal's initial value.
const PROCEDURE_DEFAULT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="procedure" name="probe_default" initial="start">
  <datamodel>
    <data id="value" sce:type="int32" sce:direction="in"/>
    <data id="counter" sce:type="int32" sce:direction="internal"
          expr="value + conut"/>
  </datamodel>
  <state id="start">
    <transition event="go" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#;

/// A procedure request's payload naming a near miss of a declared field.
const PROCEDURE_PAYLOAD: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="procedure" name="probe_payload" initial="start">
  <datamodel>
    <data id="frame" sce:type="bytes" sce:direction="in" sce:max-size="8"/>
  </datamodel>
  <state id="start">
    <onentry>
      <send sce:service="Ping"
            sce:payload="fram"/>
    </onentry>
    <transition event="ok" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#;

/// A procedure request's address naming a near miss of a declared field.
///
/// ⚠ This one used to be refused as `validation/send-operand-type` — "of no
/// type this document establishes" — because the address's type was judged
/// before anything asked whether its name is declared: the true cause went
/// unsaid and the near miss `ecuAddr` unoffered (measured 2026-09-23).
const PROCEDURE_ADDRESS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="procedure" name="probe_address" initial="start">
  <datamodel>
    <data id="ecuAddr" sce:type="uint32" sce:direction="in"/>
  </datamodel>
  <state id="start">
    <onentry>
      <send sce:service="Ping"
            sce:addr="ecuAdr"/>
    </onentry>
    <transition event="ok" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#;

/// A procedure guard written on the second row of its `<transition>` start
/// tag — the row the element starts on is not the guard's.
const PROCEDURE_COND: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="procedure" name="probe_cond" initial="start">
  <datamodel>
    <data id="retryCount" sce:type="int32" sce:direction="internal" expr="0"/>
  </datamodel>
  <state id="start">
    <transition event="go" target="done"
                cond="retryCont &lt; 3"/>
  </state>
  <final id="done"/>
</scxml>
"#;

/// An `<assign>` value, a row below the `<transition>` around it.
const PROCEDURE_ASSIGN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="procedure" name="probe_assign" initial="start">
  <datamodel>
    <data id="retryCount" sce:type="int32" sce:direction="internal" expr="0"/>
  </datamodel>
  <state id="start">
    <transition event="go" target="done">
      <assign location="retryCount"
              expr="retryCont + 1"/>
    </transition>
  </state>
  <final id="done"/>
</scxml>
"#;

/// A `<donedata>` value.
const PROCEDURE_DONEDATA: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="procedure" name="probe_donedata" initial="start">
  <datamodel>
    <data id="retryCount" sce:type="int32" sce:direction="internal" expr="0"/>
  </datamodel>
  <state id="start">
    <transition event="go" target="done"/>
  </state>
  <final id="done">
    <donedata>
      <param name="result"
             expr="retryCont"/>
    </donedata>
  </final>
</scxml>
"#;

/// An observer's leave threshold.
///
/// ⚠ This one used to be ACCEPTED: a monitor expression that did not lower
/// became the empty string, and the document was reported as generated with
/// a condition that held nothing (measured by reading the code 2026-09-23 —
/// `render_observer` was the one renderer that swallowed its lowering).
const OBSERVER_LEAVE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="observer" name="probe_observer">
  <datamodel>
    <data id="coolantTemp" sce:type="float64" sce:direction="in"/>
    <data id="warning" sce:monitor="threshold"
          sce:enter="coolantTemp &gt; 110.0"
          sce:leave="coolantTmp &lt; 100.0"
          sce:on-enter="emitWarning" sce:on-leave="clearWarning"/>
  </datamodel>
</scxml>
"#;

/// A grammar refusal whose token the document can only spell escaped: the
/// second `<` of `celsius < < 1` is written `&lt;`, and that is the text a
/// consumer finds on the row.
const TRANSFORM_ESCAPED_TOKEN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="transform" name="probe_spelling">
  <datamodel>
    <data id="celsius" sce:type="float64" sce:direction="in"/>
    <data id="result" sce:type="float64" sce:direction="out"
          expr="celsius &lt; &lt; 1"/>
  </datamodel>
</scxml>
"#;

/// A parenthesis never closed: what is refused is the end of the expression,
/// where nothing is written. The record says where the end is and reports no
/// `actual` — the parser's own word for it, `EOF`, occurs in no document.
const TRANSFORM_UNCLOSED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="transform" name="probe_unclosed">
  <datamodel>
    <data id="celsius" sce:type="float64" sce:direction="in"/>
    <data id="result" sce:type="float64" sce:direction="out"
          expr="(celsius + 1"/>
  </datamodel>
</scxml>
"#;

/// An algorithm local's initial value.
const ALGORITHM_VAR: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_alg_var" version="1.0">
  <sce:signature>
    <sce:param name="reading" type="uint16"/>
    <sce:return type="uint16"/>
  </sce:signature>
  <sce:body>
    <sce:var name="x" type="uint16"
             init="readng"/>
    <sce:return expr="x"/>
  </sce:body>
</scxml>
"#;

/// An algorithm assignment's value.
const ALGORITHM_ASSIGN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_alg_assign" version="1.0">
  <sce:signature>
    <sce:param name="reading" type="uint16"/>
    <sce:return type="uint16"/>
  </sce:signature>
  <sce:body>
    <sce:var name="x" type="uint16" init="0"/>
    <sce:assign target="x"
                expr="readng + 1"/>
    <sce:return expr="x"/>
  </sce:body>
</scxml>
"#;

/// An `<sce:append>` whose target names no buffer. The refusal names the
/// TARGET, so it is placed at that attribute, a row below the element.
const ALGORITHM_APPEND_TARGET: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_alg_append_target" version="1.0">
  <sce:signature>
    <sce:param name="reading" type="uint16"/>
    <sce:return type="bytes" returns-max-size="4"/>
  </sce:signature>
  <sce:body>
    <sce:var name="buf" type="bytes" capacity="4"/>
    <sce:append expr="1"
                target="buff"/>
    <sce:return expr="buf"/>
  </sce:body>
</scxml>
"#;

/// An `<sce:append>` of a value wider than a byte.
///
/// ⚠ This one reported the TYPE it inferred as `actual` — `uint16`, which
/// is written nowhere on the row. What the author edits is the appended
/// expression, so that is what is reported (found by reading the code
/// 2026-09-23).
const ALGORITHM_APPEND_TYPE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_alg_append_type" version="1.0">
  <sce:signature>
    <sce:param name="reading" type="uint16"/>
    <sce:return type="bytes" returns-max-size="4"/>
  </sce:signature>
  <sce:body>
    <sce:var name="buf" type="bytes" capacity="4"/>
    <sce:append target="buf"
                expr="reading"/>
    <sce:return expr="buf"/>
  </sce:body>
</scxml>
"#;

/// An `<sce:if>` guard.
const ALGORITHM_IF: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_alg_if" version="1.0">
  <sce:signature>
    <sce:param name="reading" type="uint16"/>
    <sce:return type="uint16"/>
  </sce:signature>
  <sce:body>
    <sce:if
        cond="readng &gt; 1">
      <sce:return expr="1"/>
    </sce:if>
    <sce:return expr="0"/>
  </sce:body>
</scxml>
"#;

/// An `<sce:while>` guard, the refused name after an escaped operator.
const ALGORITHM_WHILE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_alg_while" version="1.0">
  <sce:signature>
    <sce:param name="reading" type="uint16"/>
    <sce:return type="uint16"/>
  </sce:signature>
  <sce:body>
    <sce:var name="i" type="uint16" init="0"/>
    <sce:while max-iter="4"
               cond="i &lt; limt">
      <sce:assign target="i" expr="i + 1"/>
    </sce:while>
    <sce:return expr="i"/>
  </sce:body>
</scxml>
"#;

/// An `<sce:foreach>` over something that is not iterable. The refusal
/// names the SOURCE, so it is placed at the `in` attribute.
const ALGORITHM_FOREACH: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_alg_foreach" version="1.0">
  <sce:signature>
    <sce:param name="reading" type="uint16"/>
    <sce:return type="uint16"/>
  </sce:signature>
  <sce:body>
    <sce:var name="acc" type="uint16" init="0"/>
    <sce:foreach item="b"
                 in="readings">
      <sce:assign target="acc" expr="acc + 1"/>
    </sce:foreach>
    <sce:return expr="acc"/>
  </sce:body>
</scxml>
"#;

/// An `<sce:return>` value.
const ALGORITHM_RETURN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_alg_return" version="1.0">
  <sce:signature>
    <sce:param name="reading" type="uint16"/>
    <sce:return type="uint16"/>
  </sce:signature>
  <sce:body>
    <sce:return
        expr="readng"/>
  </sce:body>
</scxml>
"#;

/// An `<sce:call>` through an alias nothing imports. The refusal names the
/// ALIAS, the part of the target before its dot.
const ALGORITHM_CALL: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_alg_call" version="1.0">
  <sce:signature>
    <sce:param name="reading" type="uint16"/>
    <sce:return type="uint16"/>
  </sce:signature>
  <sce:body>
    <sce:call args="reading"
              target="telemetry.record"/>
    <sce:return expr="reading"/>
  </sce:body>
</scxml>
"#;

/// An `<sce:call>` whose second argument names a near miss, on the row its
/// `args` continues onto. The first argument holds a comma of its own, so
/// the refusal lands on the name only if `args` is split where a call's
/// argument list is.
const ALGORITHM_CALL_ARGUMENT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_alg_call_argument" version="1.0">
  <sce:import kind="algorithm" src="probe_match.scxml" as="matched"/>
  <sce:signature>
    <sce:param name="frame" type="bytes"/>
    <sce:param name="reading" type="uint16"/>
    <sce:return type="uint16"/>
  </sce:signature>
  <sce:body>
    <sce:call target="matched"
              args="frame === 'a,b',
                    readng"/>
    <sce:return expr="reading"/>
  </sce:body>
</scxml>
"#;

/// The algorithm `ALGORITHM_CALL_ARGUMENT` imports — refused nothing
/// itself, it only has to exist.
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

/// Documents a case imports, written beside the cases.
const SUPPORT: &[(&str, &str)] = &[("probe_match.scxml", PROBE_MATCH)];

const CASES: &[Case] = &[
    Case {
        file: "probe_transform.scxml",
        document: TRANSFORM_CONTINUED,
        code: "expression/unknown-identifier",
        line: 7,
        col: 19,
        actual: Some("conut"),
    },
    Case {
        file: "probe_condition.scxml",
        document: CONDITION_AFTER_AN_ESCAPE,
        code: "expression/unknown-identifier",
        line: 6,
        col: 46,
        actual: Some("conut"),
    },
    Case {
        file: "probe_validator.scxml",
        document: VALIDATOR_PLAUSIBILITY,
        code: "expression/unknown-identifier",
        line: 7,
        col: 46,
        actual: Some("conut"),
    },
    Case {
        file: "probe_default.scxml",
        document: PROCEDURE_DEFAULT,
        code: "expression/unknown-identifier",
        line: 6,
        col: 25,
        actual: Some("conut"),
    },
    Case {
        file: "probe_payload.scxml",
        document: PROCEDURE_PAYLOAD,
        code: "expression/unknown-identifier",
        line: 9,
        col: 26,
        actual: Some("fram"),
    },
    Case {
        file: "probe_address.scxml",
        document: PROCEDURE_ADDRESS,
        code: "expression/unknown-identifier",
        line: 9,
        col: 23,
        actual: Some("ecuAdr"),
    },
    Case {
        file: "probe_cond.scxml",
        document: PROCEDURE_COND,
        code: "expression/unknown-identifier",
        line: 8,
        col: 23,
        actual: Some("retryCont"),
    },
    Case {
        file: "probe_assign.scxml",
        document: PROCEDURE_ASSIGN,
        code: "expression/unknown-identifier",
        line: 9,
        col: 21,
        actual: Some("retryCont"),
    },
    Case {
        file: "probe_donedata.scxml",
        document: PROCEDURE_DONEDATA,
        code: "expression/unknown-identifier",
        line: 12,
        col: 20,
        actual: Some("retryCont"),
    },
    Case {
        file: "probe_observer.scxml",
        document: OBSERVER_LEAVE,
        code: "expression/unknown-identifier",
        line: 7,
        col: 22,
        actual: Some("coolantTmp"),
    },
    Case {
        file: "probe_spelling.scxml",
        document: TRANSFORM_ESCAPED_TOKEN,
        code: "expression/unexpected-token",
        line: 6,
        col: 30,
        actual: Some("&lt;"),
    },
    Case {
        file: "probe_unclosed.scxml",
        document: TRANSFORM_UNCLOSED,
        code: "expression/parse-mismatch",
        line: 6,
        col: 29,
        actual: None,
    },
    Case {
        file: "probe_alg_var.scxml",
        document: ALGORITHM_VAR,
        code: "expression/unknown-identifier",
        line: 9,
        col: 20,
        actual: Some("readng"),
    },
    Case {
        file: "probe_alg_assign.scxml",
        document: ALGORITHM_ASSIGN,
        code: "expression/unknown-identifier",
        line: 10,
        col: 23,
        actual: Some("readng"),
    },
    Case {
        file: "probe_alg_append_target.scxml",
        document: ALGORITHM_APPEND_TARGET,
        code: "algorithm/append-target-not-buffer",
        line: 10,
        col: 25,
        actual: Some("buff"),
    },
    Case {
        file: "probe_alg_append_type.scxml",
        document: ALGORITHM_APPEND_TYPE,
        code: "algorithm/append-type-mismatch",
        line: 10,
        col: 23,
        actual: Some("reading"),
    },
    Case {
        file: "probe_alg_if.scxml",
        document: ALGORITHM_IF,
        code: "expression/unknown-identifier",
        line: 9,
        col: 15,
        actual: Some("readng"),
    },
    Case {
        file: "probe_alg_while.scxml",
        document: ALGORITHM_WHILE,
        code: "expression/unknown-identifier",
        line: 10,
        col: 29,
        actual: Some("limt"),
    },
    Case {
        file: "probe_alg_foreach.scxml",
        document: ALGORITHM_FOREACH,
        code: "algorithm/foreach-source-not-iterable",
        line: 10,
        col: 22,
        actual: Some("readings"),
    },
    Case {
        file: "probe_alg_return.scxml",
        document: ALGORITHM_RETURN,
        code: "expression/unknown-identifier",
        line: 9,
        col: 15,
        actual: Some("readng"),
    },
    Case {
        file: "probe_alg_call.scxml",
        document: ALGORITHM_CALL,
        code: "algorithm/call-target-unknown",
        line: 9,
        col: 23,
        actual: Some("telemetry"),
    },
    Case {
        file: "probe_alg_call_argument.scxml",
        document: ALGORITHM_CALL_ARGUMENT,
        code: "expression/unknown-identifier",
        line: 12,
        col: 21,
        actual: Some("readng"),
    },
];

/// Every case and every document a case imports, written into one
/// directory.
fn fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let cases = CASES.iter().map(|case| (case.file, case.document));
    for (file, document) in cases.chain(SUPPORT.iter().copied()) {
        std::fs::write(dir.path().join(file), document).expect("write document");
    }
    dir
}

/// The one record `sce-codegen` prints for `args` in JSON mode, or why
/// there is not exactly one.
fn refusal(dir: &Path, args: &[&str]) -> Result<serde_json::Value, String> {
    let output = Command::new(codegen_bin())
        .current_dir(dir)
        .arg("--error-format=json")
        .args(args)
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
            "location.file",
            record["location"]["file"].clone(),
            serde_json::json!(case.file),
        ),
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
fn every_backend_names_the_row_and_column_of_the_refused_token() {
    let dir = fixture();
    let mut failures = Vec::new();
    for case in CASES {
        for &lang in Language::ALL {
            let wire = language_wire_name(lang);
            // Go refuses any `<sce:import>` without the module path its
            // generated imports are qualified by (`generate/invalid-config`),
            // before a single expression is lowered; the others ignore it.
            let check = [
                "check",
                case.file,
                "-l",
                wire,
                "--go-module-prefix",
                "example.com/probe",
            ];
            let wrong = match refusal(dir.path(), &check) {
                Ok(record) => mismatches(case, &record),
                Err(why) => vec![why],
            };
            if !wrong.is_empty() {
                failures.push(format!("{} [{wire}]: {}", case.file, wrong.join("; ")));
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

/// The library facade places the refusal as the CLI does: the row and the
/// column travel on the error itself, not only on the wire record.
#[test]
fn the_library_route_places_the_refusal_as_the_cli_does() {
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
