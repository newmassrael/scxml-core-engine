// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The Swift backend lowers a statechart of `datamodel="ecmascript"` whose only uses of a script
//! engine are `<data>` initialisers that are expressions and transition `cond`s: both are lowered to
//! Lua at build time and run by the `SceLua` engine (docs/SCE_LUA_TRANSLATION_SEAM.md), the engine
//! every other Lua backend runs the same Lua on. Every other reason a document needs an engine is
//! refused by name and not written without its engine call.
//!
//! `EcmaScriptMachinesTests` holds what the machines do under their `Engine`; this file holds the
//! text the generator writes and the line it refuses by name.
//!
//! # What was wrong
//!
//! The Swift generator refused `datamodel="ecmascript"` whole ("would need a script engine"), which
//! is the default data model, so a document that omitted the attribute was refused too, and 247 of
//! the 252 W3C documents could not be lowered. The engine existed in every other backend and in
//! Swift only after the `SceLua` module.

use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::tempdir;

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// A document: the attributes of `<scxml>` beyond the fixed ones, and the elements inside it.
fn document(attributes: &str, body: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s0" name="probe" {attributes}>
{body}
</scxml>
"#
    )
}

/// What `sce-codegen generate -l swift` made of `text`, written as `probe.scxml` beside the files of
/// `extra`: its exit status, the text of the machine it wrote (empty when it wrote none), and
/// everything it said.
fn generate(text: &str, extra: &[(&str, &str)]) -> (Option<i32>, String, String) {
    let dir = tempdir().expect("tempdir");
    std::fs::write(dir.path().join("probe.scxml"), text).expect("write the document");
    for (name, content) in extra {
        std::fs::write(dir.path().join(name), content).expect("write a file beside it");
    }
    let out_dir = tempdir().expect("tempdir");
    let out = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .args(["generate", "-l", "swift"])
        .arg(dir.path().join("probe.scxml"))
        .arg("-o")
        .arg(out_dir.path())
        .output()
        .expect("invoke sce-codegen");
    let machine =
        std::fs::read_to_string(out_dir.path().join("probe_sm.swift")).unwrap_or_default();
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.code(), machine, said)
}

const GUARDED: &str = r#"
  <datamodel><data id="n" expr="1"/></datamodel>
  <state id="s0"><transition cond="n == 1" target="done"/></state>
  <final id="done"/>"#;

#[test]
fn a_guard_and_a_data_initialiser_are_lowered_to_lua_and_run_by_the_engine() {
    let (status, machine, said) = generate(&document(r#"datamodel="ecmascript""#, GUARDED), &[]);
    assert_eq!(status, Some(0), "{said}");
    assert!(machine.contains("import SceLua"), "{machine}");
    assert!(
        machine.contains(r#"defineData("n", as: "1", engine: engine)"#),
        "the initialiser is lowered and handed to the session: {machine}"
    );
    assert!(
        machine.contains(r#"evaluateGuard("_scxml_eq(n, 1)", engine: engine)"#),
        "the cond is lowered to Lua and asked of the engine: {machine}"
    );
    assert!(
        machine.contains("public var needsScriptEngine: Bool { true }")
            && machine.contains("public var needsDataModelInit: Bool { true }"),
        "{machine}"
    );
}

/// The default data model is `ecmascript`, so a document that omits the attribute is lowered the same.
#[test]
fn a_document_that_omits_its_data_model_is_an_ecmascript_one() {
    let (status, machine, said) = generate(&document("", GUARDED), &[]);
    assert_eq!(status, Some(0), "{said}");
    assert!(machine.contains("import SceLua"), "{machine}");
}

/// A machine that needs no engine does not link Lua: `SceRuntime` does not import `SceLua`, and the
/// machine imports it only when the document asks.
#[test]
fn a_machine_that_needs_no_engine_does_not_import_the_engine() {
    let body = r#"<state id="s0"><transition event="go" target="done"/></state><final id="done"/>"#;
    let (status, machine, said) = generate(&document(r#"datamodel="null""#, body), &[]);
    assert_eq!(status, Some(0), "{said}");
    assert!(!machine.contains("import SceLua"), "{machine}");
    assert!(!machine.contains("evaluateGuard"), "{machine}");
    assert!(
        machine.contains("public var needsScriptEngine: Bool { false }"),
        "{machine}"
    );
}

/// A cond the frontend decided is a value written into the machine; the engine is not asked what
/// the build already knows.
#[test]
fn a_cond_the_build_decided_is_not_sent_to_the_engine() {
    let body = r#"
  <datamodel><data id="n" expr="1"/></datamodel>
  <state id="s0">
    <transition cond="true" target="done"/>
  </state>
  <final id="done"/>"#;
    let (status, machine, said) = generate(&document(r#"datamodel="ecmascript""#, body), &[]);
    assert_eq!(status, Some(0), "{said}");
    assert!(!machine.contains("evaluateGuard(\""), "{machine}");
}

/// The name of a `<data>` crosses as a string: an XML name is not always a Lua identifier.
#[test]
fn the_name_of_a_data_variable_is_a_string_and_never_source() {
    let body = r#"
  <datamodel><data id="screen-rules" expr="(4)"/></datamodel>
  <state id="s0"><transition cond="true" target="done"/></state>
  <final id="done"/>"#;
    let (status, machine, said) = generate(&document(r#"datamodel="ecmascript""#, body), &[]);
    assert_eq!(status, Some(0), "{said}");
    assert!(
        machine.contains(r#"defineData("screen-rules", as: "#),
        "{machine}"
    );
}

/// A `<send>` and a `<cancel>` of literals need no engine: the event, the id and the `<param>` the
/// document writes as text are read by the template, so the machine delivers them whichever data
/// model the document declares.
#[test]
fn a_send_and_a_cancel_of_literals_are_lowered() {
    // A `<param expr>` is an expression, which a `null` data model has no language to write in, so
    // that data model carries the literal as `<content>` and the other as a `<param>`.
    for (datamodel, data) in [
        ("null", r#"<content>me</content>"#),
        ("ecmascript", r#"<param name="who" expr="'me'"/>"#),
    ] {
        let body = format!(
            r#"
  <state id="s0">
    <onentry>
      <send id="first" event="ping">{data}</send>
      <cancel sendid="first"/>
      <send event="pong"/>
    </onentry>
    <transition event="pong" target="done"/>
  </state>
  <final id="done"/>"#
        );
        let (status, machine, said) = generate(
            &document(&format!(r#"datamodel="{datamodel}""#), &body),
            &[],
        );
        assert_eq!(status, Some(0), "{datamodel}: {said}");
        assert_eq!(
            machine.matches("engine.raiseExternalWithMeta(").count(),
            2,
            "{datamodel}: each of the two sends is delivered through the engine: {machine}"
        );
        assert!(
            machine.contains("engine.cancelEvent("),
            "{datamodel}: the cancel reaches the engine: {machine}"
        );
    }
}

/// A BasicHTTP `<send>` that names no target has nowhere to go: the build decides it once, and the
/// machine raises error.communication (W3C SCXML C.2, W3C 577) instead of handing the engine's
/// transport a request for the empty string.
#[test]
fn a_basichttp_send_with_no_target_is_an_error_and_reaches_no_transport() {
    let body = r#"
  <state id="s0">
    <onentry><send event="t" type="http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor"/></onentry>
    <transition event="error.communication" target="done"/>
  </state>
  <final id="done"/>"#;
    let (status, machine, said) = generate(&document(r#"datamodel="ecmascript""#, body), &[]);
    assert_eq!(status, Some(0), "{said}");
    assert!(
        machine.contains("BasicHTTP names no target"),
        "the send raises the error: {machine}"
    );
    assert!(
        !machine.contains("engine.performHTTPSend("),
        "and hands no transport a request: {machine}"
    );
}

/// The text of a `<content>` travels as it was written under `ecmascript`, for the receiver's engine
/// to read as JSON or as a string (§scxml-B-2-8-1, W3C 578); under any other data model it is the
/// value, the last rung taken at build time.
#[test]
fn the_content_of_a_send_travels_as_written_for_the_receivers_engine() {
    let body = r#"
  <state id="s0">
    <onentry><send event="e"><content>{ "a" : 1 }</content></send></onentry>
    <transition event="e" target="done"/>
  </state>
  <final id="done"/>"#;
    let written = r##"let sendEventData = "{ \"a\" : 1 }""##;

    let (status, machine, said) = generate(&document(r#"datamodel="ecmascript""#, body), &[]);
    assert_eq!(status, Some(0), "{said}");
    assert!(machine.contains(written), "{machine}");

    let (status, machine, said) = generate(&document(r#"datamodel="null""#, body), &[]);
    assert_eq!(status, Some(0), "{said}");
    assert!(
        !machine.contains(written),
        "a null data model has no engine to read it, so the value is the string: {machine}"
    );
}

/// `#_<invokeid>` addresses an invoked child. Delivering it to the sender's own queue, as the
/// targets that name the sender are, would hand the child's event to its parent (W3C 347 waited
/// for a `done.invoke` that way), so it is routed by the engine's target table.
#[test]
fn a_send_to_an_invoked_child_is_routed_and_not_kept_by_the_sender() {
    let body = r##"
  <state id="s0">
    <invoke id="child" type="scxml">
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="c0" datamodel="ecmascript">
          <state id="c0"><transition event="go" target="cdone"/></state>
          <final id="cdone"/>
        </scxml>
      </content>
    </invoke>
    <onentry><send target="#_child" event="go"/></onentry>
    <transition event="done.invoke" target="done"/>
  </state>
  <final id="done"/>"##;
    let (status, machine, said) = generate(&document(r#"datamodel="ecmascript""#, body), &[]);
    assert_eq!(status, Some(0), "{said}");
    assert!(
        machine.contains(r##"target: "#_child","##),
        "the send is routed by its target: {machine}"
    );
    assert!(machine.contains("engine.sendToTarget("), "{machine}");
}

/// An `<assign>` of an `ecmascript` document is lowered to Lua: its location and its value are
/// lowered at build time and the engine runs the assignment, an error of which is
/// `error.execution` and ends the block.
#[test]
fn an_assign_is_lowered_to_lua_for_the_engine() {
    let body = r#"
  <datamodel><data id="n" expr="1"/></datamodel>
  <state id="s0">
    <onentry><assign location="n" expr="n + 1"/></onentry>
    <transition cond="n == 2" target="done"/>
  </state>
  <final id="done"/>"#;
    let (status, machine, said) = generate(&document(r#"datamodel="ecmascript""#, body), &[]);
    assert_eq!(status, Some(0), "{said}");
    assert!(machine.contains(r#"assignData("n", "#), "{machine}");
    assert!(
        machine.contains("<assign> to 'n' failed"),
        "a failed assignment is an error.execution: {machine}"
    );
}

/// The system variables are read-only, decided once for every backend: an assign to one is an
/// `error.execution` at the point it is reached and reaches no engine.
#[test]
fn an_assign_to_a_system_variable_is_an_error_and_reaches_no_engine() {
    let body = r#"
  <state id="s0">
    <onentry><assign location="_sessionid" expr="1"/></onentry>
    <transition event="error.execution" target="done"/>
  </state>
  <final id="done"/>"#;
    let (status, machine, said) = generate(&document(r#"datamodel="ecmascript""#, body), &[]);
    assert_eq!(status, Some(0), "{said}");
    assert!(
        machine.contains("invalid or read-only location '_sessionid'"),
        "{machine}"
    );
    assert!(
        !machine.contains(r#"assignData("_sessionid""#),
        "no assignment is written: {machine}"
    );
}

/// Each cond of an `<if>` chain of an `ecmascript` document is lowered to Lua and run by the
/// engine, which says whether it failed: a cond that cannot be run is false and ends the block.
#[test]
fn an_if_chain_is_lowered_to_lua_and_a_failed_cond_ends_the_block() {
    let body = r#"
  <datamodel><data id="n" expr="1"/></datamodel>
  <state id="s0">
    <onentry>
      <if cond="n == 1"><raise event="one"/>
      <elseif cond="n == 2"/><raise event="two"/>
      <else/><raise event="other"/>
      </if>
    </onentry>
    <transition event="one" target="done"/>
  </state>
  <final id="done"/>"#;
    let (status, machine, said) = generate(&document(r#"datamodel="ecmascript""#, body), &[]);
    assert_eq!(status, Some(0), "{said}");
    assert!(
        machine.contains(r#"if evaluateIfCond(""#)
            && machine.contains("failed: &ifCondFailed1, engine: engine) {"),
        "the cond is run by the engine, with the flag it sets: {machine}"
    );
    assert!(
        machine.contains(r#"} else if evaluateIfCond(""#),
        "an <elseif> is run by the engine too, under the chain's flag: {machine}"
    );
    assert_eq!(
        machine.matches("failed: &ifCondFailed1").count(),
        2,
        "the <if> and its <elseif> share one flag: {machine}"
    );
    assert!(
        machine.contains("if ifCondFailed1 {"),
        "a chain that failed ends its block: {machine}"
    );
}

/// An inline `<script>` is lowered to Lua and run by the engine; a body that fails is an
/// `error.execution` and ends the block.
#[test]
fn an_inline_script_is_lowered_to_lua_for_the_engine() {
    let body = r#"
  <datamodel><data id="n" expr="1"/></datamodel>
  <state id="s0">
    <onentry><script>n = n + 1;</script></onentry>
    <transition cond="n == 2" target="done"/>
  </state>
  <final id="done"/>"#;
    let (status, machine, said) = generate(&document(r#"datamodel="ecmascript""#, body), &[]);
    assert_eq!(status, Some(0), "{said}");
    assert!(machine.contains(r#"executeScript(""#), "{machine}");
    assert!(
        machine.contains("<script> failed to execute"),
        "a failed script is an error.execution: {machine}"
    );
    assert!(
        machine.contains("break sceBlock"),
        "the error ends the block: {machine}"
    );
}

/// A top-level `<script>` runs once, at document load, after the data is declared.
#[test]
fn a_top_level_script_runs_at_load_after_the_data() {
    let body = r#"
  <datamodel><data id="n" expr="1"/></datamodel>
  <script>n = n + 10;</script>
  <state id="s0"><transition cond="n == 11" target="done"/></state>
  <final id="done"/>"#;
    let (status, machine, said) = generate(&document(r#"datamodel="ecmascript""#, body), &[]);
    assert_eq!(status, Some(0), "{said}");
    let data = machine
        .find(r#"defineData("n""#)
        .expect("the data is declared");
    let script = machine
        .find("a top-level <script> failed to execute")
        .expect("the top-level script is run");
    assert!(data < script, "the script runs after the data: {machine}");
}

/// A cond the build decided is not sent to the engine, so an `<if>` on one has no flag to keep.
#[test]
fn an_if_on_a_cond_the_build_decided_keeps_no_flag() {
    let body = r#"
  <datamodel><data id="n" expr="1"/></datamodel>
  <state id="s0">
    <onentry><if cond="true"><raise event="e"/></if></onentry>
    <transition event="e" target="done"/>
  </state>
  <final id="done"/>"#;
    let (status, machine, said) = generate(&document(r#"datamodel="ecmascript""#, body), &[]);
    assert_eq!(status, Some(0), "{said}");
    assert!(!machine.contains("ifCondFailed"), "{machine}");
    assert!(!machine.contains("evaluateIfCond(\""), "{machine}");
}

/// What a `<send>` computes with an expression is still refused by name.
#[test]
fn a_send_that_an_expression_computes_is_refused_by_name() {
    for (what, send) in [
        ("an eventexpr", r#"<send eventexpr="'x'"/>"#),
        (
            "a targetexpr",
            r#"<send event="x" targetexpr="'#_internal'"/>"#,
        ),
        ("a delayexpr", r#"<send event="x" delayexpr="'1s'"/>"#),
        ("a namelist", r#"<send event="x" namelist="n"/>"#),
        (
            "a param expression",
            r#"<send event="x"><param name="p" expr="n + 1"/></send>"#,
        ),
        (
            "a content expression",
            r#"<send event="x"><content expr="n"/></send>"#,
        ),
    ] {
        let body = format!(
            r#"<datamodel><data id="n" expr="1"/></datamodel>
  <state id="s0"><onentry>{send}</onentry></state>"#
        );
        let (status, machine, said) = generate(&document(r#"datamodel="ecmascript""#, &body), &[]);
        assert_eq!(
            status,
            Some(7),
            "{what} must be refused, not written: {said}"
        );
        assert!(machine.is_empty(), "{what} wrote a machine anyway");
        assert!(
            said.contains("<send") && said.contains("no Swift codegen path yet"),
            "{what} is refused, but not by name: {said}"
        );
    }
}

/// Each reason a document needs the engine that the templates do not lower is refused by its name,
/// with the status that says it is an unsupported feature.
#[test]
fn what_the_templates_do_not_lower_is_refused_by_name() {
    struct Refusal {
        what: &'static str,
        attributes: &'static str,
        body: &'static str,
        extra: &'static [(&'static str, &'static str)],
        named: &'static str,
    }
    let refusals = [
        Refusal {
            what: "a <foreach>",
            attributes: r#"datamodel="ecmascript""#,
            body: r#"<datamodel><data id="l" expr="[1]"/><data id="i"/></datamodel>
  <state id="s0"><onentry><foreach array="l" item="i"><raise event="e"/></foreach></onentry></state>"#,
            extra: &[],
            named: "<foreach>",
        },
        Refusal {
            what: "a <log expr>",
            attributes: r#"datamodel="ecmascript""#,
            body: r#"<state id="s0"><onentry><log expr="'x'"/></onentry></state>"#,
            extra: &[],
            named: "<log>",
        },
        Refusal {
            what: "a <data src>",
            attributes: r#"datamodel="ecmascript""#,
            body: r#"<datamodel><data id="x" src="probe.json"/></datamodel><state id="s0"/>"#,
            extra: &[("probe.json", r#"{"a":1}"#)],
            named: "a <data src>",
        },
        Refusal {
            what: "an inline <data> content",
            attributes: r#"datamodel="ecmascript""#,
            body: r#"<datamodel><data id="x">42</data></datamodel><state id="s0"/>"#,
            extra: &[],
            named: "an inline <data> content",
        },
        Refusal {
            what: "a function expression as an initialiser",
            attributes: r#"datamodel="ecmascript""#,
            body: r#"<datamodel><data id="f" expr="function() { return 1; }"/></datamodel><state id="s0"/>"#,
            extra: &[],
            named: "a function expression",
        },
        Refusal {
            what: "late binding of a state's own data",
            attributes: r#"datamodel="ecmascript" binding="late""#,
            body: r#"<state id="s0"><datamodel><data id="x" expr="1"/></datamodel></state>"#,
            extra: &[],
            named: r#"binding="late""#,
        },
        Refusal {
            what: "a <donedata> of a final state",
            attributes: r#"datamodel="ecmascript""#,
            body: r#"<datamodel><data id="n" expr="1"/></datamodel>
  <state id="s0"><transition target="done"/></state>
  <final id="done"><donedata><content expr="n"/></donedata></final>"#,
            extra: &[],
            named: "donedata",
        },
    ];
    for refusal in refusals {
        let (status, machine, said) =
            generate(&document(refusal.attributes, refusal.body), refusal.extra);
        assert_eq!(
            status,
            Some(7),
            "{} must be refused as an unsupported feature, not written: {said}",
            refusal.what
        );
        assert!(
            machine.is_empty(),
            "{} wrote a machine anyway: {machine}",
            refusal.what
        );
        assert!(
            said.contains(refusal.named) && said.contains("no Swift codegen path yet"),
            "{} is refused, but not by name ({}): {said}",
            refusal.what,
            refusal.named
        );
    }
}
