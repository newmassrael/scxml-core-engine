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

/// W3C SCXML 5.7.1: a `<send>`'s `<param>` whose `location` is empty names no location, so it raises
/// `error.execution`, leaves its pair out and ends its block. The template skipped it without a word
/// (measured 2026-11-01 by the integration stem `a_bad_send_param_ends_its_block`: two of three errors
/// raised and the block that held the third ran on).
#[test]
fn a_send_param_with_an_empty_location_is_an_error_that_ends_its_block() {
    let body = r##"
  <state id="s0">
    <onentry>
      <send event="e"><param name="p" location=""/></send>
      <assign location="after" expr="1"/>
    </onentry>
  </state>"##;
    let (status, machine, said) = generate(&document(r#"datamodel="ecmascript""#, body), &[]);
    assert_eq!(status, Some(0), "{said}");
    assert!(
        machine.contains("<send> <param name=p> names no location")
            && machine.contains("paramFailed = true"),
        "an empty location is reported and the block is ended: {machine}"
    );
}

/// W3C SCXML B.2.8.1: which rung a payload got is known only where `_event` is bound, so the machine
/// tells the engine there. Without the call `Engine.undecodablePayloads` stays 0 for every
/// ecmascript machine and a payload a host sent as structure is lost to the document and to the host
/// alike (measured 2026-11-01 by the integration stem `undecodable_payload_is_reported`).
#[test]
fn the_engine_is_told_which_rung_a_payload_got_when_the_event_is_bound() {
    let (status, machine, said) = generate(&document(r#"datamodel="ecmascript""#, GUARDED), &[]);
    assert_eq!(status, Some(0), "{said}");
    assert!(
        machine.contains("let reading = try scriptSession.bindCurrentEvent("),
        "the reading the script engine reports is kept: {machine}"
    );
    assert!(
        machine.contains(
            "engine.notePayloadReading(event, reading: Self.payloadReading(of: reading))"
        ),
        "and handed to the engine, which counts the undecodable one: {machine}"
    );
}

/// docs/adr/0096: the session is made in one place, which the engine asks first in every start, and
/// every later use reads the session that start made. The seam a test fails a step through is a debug
/// build's alone, so a release build of a machine cannot be made to refuse its start on request.
#[test]
fn the_session_is_established_by_the_start_and_failed_on_request_only_in_a_debug_build() {
    let (status, machine, said) = generate(&document(r#"datamodel="ecmascript""#, GUARDED), &[]);
    assert_eq!(status, Some(0), "{said}");
    assert!(
        machine.contains("public func establishScriptSession() -> ScriptSessionFailure? {"),
        "the session is established where the engine asks for it: {machine}"
    );
    assert!(
        !machine.contains("ensureLua"),
        "nothing makes a session at its first use: {machine}"
    );
    let seam = machine
        .find("\n#if DEBUG\n    /// Asked before each step of establishing a session")
        .unwrap_or_else(|| panic!("the seam is compiled into a debug build: {machine}"));
    let release_step = concat!(
        "\n#else\n",
        "    private func sceScriptSessionFaultInjected(at stage: ScriptSessionStage) throws {}\n",
        "#endif\n"
    );
    let release = machine[seam..]
        .find(release_step)
        .unwrap_or_else(|| panic!("a release build's step asks nothing: {machine}"));
    assert!(
        !machine[seam + release..].contains("sceScriptSessionFault?("),
        "the release branch reads no seam: {machine}"
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

/// The text of a `<content>` is the value its data model reads it as, decided at build time by the
/// one reading every backend takes (`filters::inline_content_reading`). Under `ecmascript` the text
/// is read first as an expression, so `{ "a" : 1 }` is an object literal and the event carries the
/// object as the JSON every engine writes it in, members sorted and no layout, which the receiver
/// reads back as the object (§scxml-B-2-8-1); W3C 578's receiver reads `_event.data.productName`
/// from exactly that. The text as written would
/// read the same at the receiver, but only where the receiver parses it as JSON, and it would be a
/// second spelling of one value that no other backend writes. Under `null` there is no value
/// expression language, so the value is the string the text spells.
#[test]
fn the_content_of_a_send_is_the_value_its_data_model_reads() {
    let body = r#"
  <state id="s0">
    <onentry><send event="e"><content>{ "a" : 1 }</content></send></onentry>
    <transition event="e" target="done"/>
  </state>
  <final id="done"/>"#;
    let object = r##"let sendEventData = "{\"a\":1}""##;
    let string = r##"let sendEventData = "\"{ \\\"a\\\" : 1 }\"""##;

    let (status, machine, said) = generate(&document(r#"datamodel="ecmascript""#, body), &[]);
    assert_eq!(status, Some(0), "{said}");
    assert!(
        machine.contains(object),
        "under ecmascript the content is the object it denotes: {machine}"
    );

    let (status, machine, said) = generate(&document(r#"datamodel="null""#, body), &[]);
    assert_eq!(status, Some(0), "{said}");
    assert!(
        machine.contains(string),
        "a null data model has no expressions, so the value is the string: {machine}"
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

/// What an `<invoke>` hands a child that runs a script engine: each `<param>` and each namelist name
/// is read once by the parent's engine, as the value itself when it is a scalar and as its JSON
/// otherwise (JSON writes `Infinity` as `null`, so a value that went by its JSON reached the child as
/// something else: measured 2026-11-01 by the integration stem `invoke_param_seeds_declared_child_data`),
/// and given to the child by name, and the child's `<data>` of that name takes it. The id of the
/// invocation goes to the `idlocation` before the child starts. A namelist name the document does not
/// declare ends the element instead (`a_namelist_naming_an_undeclared_variable_ends_the_invoke_with_one_error`).
#[test]
fn an_invoke_hands_its_child_the_values_it_names_as_values() {
    let body = r##"
  <datamodel><data id="n" expr="1"/><data id="m" expr="2"/><data id="where"/></datamodel>
  <state id="s0">
    <invoke type="scxml" idlocation="where" namelist="n">
      <param name="p" expr="m + 1"/>
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="c0" datamodel="ecmascript">
          <datamodel><data id="p" expr="0"/><data id="n" expr="0"/></datamodel>
          <state id="c0"><transition target="cdone"/></state>
          <final id="cdone"/>
        </scxml>
      </content>
    </invoke>
    <transition event="done.invoke" target="done"/>
  </state>
  <final id="done"/>"##;
    let (status, machine, said) = generate(&document(r#"datamodel="ecmascript""#, body), &[]);
    assert_eq!(status, Some(0), "{said}");
    assert!(
        machine.contains(r#"seeds["p"] = seedValue"#)
            && machine.contains(r#"seeds["n"] = seedValue"#),
        "a param and a namelist name are handed by name: {machine}"
    );
    assert!(machine.contains("acceptSeeds(seeds)"), "{machine}");
    assert!(
        machine.contains("var seeds: [String: LuaSeed] = [:]")
            && machine.contains(r#"if let seedValue = evaluateSeed(""#)
            && !machine.contains("evaluateJSON(\"m"),
        "a value is read once and kept as a value, not rendered as JSON first: {machine}"
    );
    assert!(
        machine.contains(r#"assignData("where", ""#),
        "the id of the invocation is written to the idlocation: {machine}"
    );
    assert!(
        !machine.contains("is not declared by this document"),
        "no build-time refusal is embedded for a name: {machine}"
    );
}

/// The `<finalize>` of an `<invoke>` is the script the parent runs, in its own data model with `_event`
/// bound to the event the child sent, before it selects transitions on that event. The invocation is
/// the one whose session the event names as its origin, so the script rides on the session the parent
/// tracks, and the machine reports that it has a finalize.
#[test]
fn a_finalize_runs_in_the_parent_before_it_selects_on_the_childs_event() {
    let body = r##"
  <datamodel><data id="seen" expr="0"/></datamodel>
  <state id="s0">
    <invoke type="scxml">
      <finalize><assign location="seen" expr="_event.data"/></finalize>
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="c0" datamodel="ecmascript">
          <datamodel><data id="p" expr="0"/></datamodel>
          <state id="c0"><transition target="cdone"/></state>
          <final id="cdone"/>
        </scxml>
      </content>
    </invoke>
    <transition event="done.invoke" target="done"/>
  </state>
  <final id="done"/>"##;
    let (status, machine, said) = generate(&document(r#"datamodel="ecmascript""#, body), &[]);
    assert_eq!(status, Some(0), "{said}");
    assert!(
        machine.contains("public var hasFinalize: Bool { true }"),
        "the machine reports that it has a finalize: {machine}"
    );
    assert!(
        machine.contains("public func executeFinalizeForChildEvent("),
        "the finalize is run for the child's event: {machine}"
    );
    assert!(
        machine.contains(r#"finalizeScript: ""#) && !machine.contains(r#"finalizeScript: """#),
        "the session the parent tracks carries the finalize script: {machine}"
    );
    assert!(
        machine.contains("bindCurrentEvent(event.event, engine: engine)"),
        "_event is the event the child sent: {machine}"
    );
}

/// A child that runs no script engine has no variable to take a value, so what is handed to it is
/// read, for the errors an argument can cost, and not given: the child is never offered a seed.
#[test]
fn values_handed_to_a_child_without_an_engine_are_read_and_not_given() {
    let body = r##"
  <datamodel><data id="n" expr="1"/></datamodel>
  <state id="s0">
    <invoke type="scxml" namelist="n">
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="c0">
          <state id="c0"/>
        </scxml>
      </content>
    </invoke>
  </state>"##;
    let (status, machine, said) = generate(&document(r#"datamodel="ecmascript""#, body), &[]);
    assert_eq!(status, Some(0), "{said}");
    assert!(
        machine.contains(r#"evaluateSeed("n")"#),
        "the argument is read: {machine}"
    );
    assert!(
        !machine.contains("childPolicy.acceptSeeds("),
        "a child with no engine is given nothing: {machine}"
    );
}

/// §scxml-6.4 + SCE Accepted Subset §2.13: an `<invoke srcexpr>` that declares no `sce:candidates`
/// has no finite set of children to choose among, so its child is the stub the build writes for it
/// (an immediate `<final>`); the expression is read to classify its errors, and a failure to read it
/// is `error.execution` with nothing started.
#[test]
fn an_invoke_srcexpr_with_no_candidates_reads_the_expression_and_starts_the_stub() {
    let body = r##"
  <datamodel><data id="where" expr="'x.scxml'"/></datamodel>
  <state id="s0">
    <invoke type="scxml" srcexpr="where"/>
    <transition event="done.invoke" target="done"/>
  </state>
  <final id="done"/>"##;
    let (status, machine, said) = generate(&document(r#"datamodel="ecmascript""#, body), &[]);
    assert_eq!(status, Some(0), "{said}");
    assert!(
        machine.contains(r#"guard evaluateText("where") != nil else {"#),
        "the expression is read, and a failure to read it ends the start: {machine}"
    );
    assert!(
        machine.contains("ProbeHybrid0Policy()"),
        "the child is the stub of the invoke: {machine}"
    );
    assert!(
        !machine.contains("documentStem("),
        "no candidate is chosen by the value: {machine}"
    );
}

/// §scxml-6.4: a namelist name that is not a declared variable is not a readable location, which
/// ends the element: one error.execution however many names are bad, and no session starts.
#[test]
fn a_namelist_naming_an_undeclared_variable_ends_the_invoke_with_one_error() {
    let body = r##"
  <datamodel><data id="n" expr="1"/></datamodel>
  <state id="s0">
    <invoke type="scxml" namelist="missingA missingB">
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="c0">
          <state id="c0"/>
        </scxml>
      </content>
    </invoke>
  </state>"##;
    let (status, machine, said) = generate(&document(r#"datamodel="ecmascript""#, body), &[]);
    assert_eq!(status, Some(0), "{said}");
    assert_eq!(
        machine
            .matches("<invoke> namelist names a location that cannot be read")
            .count(),
        1,
        "one error for the element, however many names are bad: {machine}"
    );
    assert!(
        !machine.contains("InvokedSession("),
        "no session is started for it: {machine}"
    );
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

/// A `<log expr>` is lowered to Lua and shown as ECMAScript converts the value to a string; an
/// expression that fails is an `error.execution` and ends the block, and a label alone is shown as
/// it always was.
#[test]
fn a_log_expression_is_lowered_to_lua_and_shown_as_a_string() {
    let body = r#"
  <datamodel><data id="n" expr="1"/></datamodel>
  <state id="s0">
    <onentry>
      <log label="count" expr="n + 1"/>
      <log label="plain"/>
    </onentry>
    <transition target="done"/>
  </state>
  <final id="done"/>"#;
    let (status, machine, said) = generate(&document(r#"datamodel="ecmascript""#, body), &[]);
    assert_eq!(status, Some(0), "{said}");
    assert!(
        machine.contains(r#"if let logged = evaluateText(""#),
        "the expression is run by the engine: {machine}"
    );
    assert!(
        machine.contains(r#"print("count: " + logged)"#),
        "the label leads the value: {machine}"
    );
    assert!(
        machine.contains("<log> expr failed to evaluate"),
        "a failed expression is an error.execution: {machine}"
    );
    assert!(
        machine.contains(r#"print("plain")"#),
        "a label alone is printed as before: {machine}"
    );
}

/// A `<foreach>` reads its array once through the engine and walks a native loop, binding the item and
/// the index to each element; a nested one keeps its own array, and an array the engine cannot walk
/// is an `error.execution` that ends the block holding the loop.
#[test]
fn a_foreach_walks_an_array_the_engine_read_once() {
    let body = r#"
  <datamodel><data id="outer" expr="[1, 2]"/><data id="inner" expr="[3]"/><data id="i"/><data id="j"/><data id="k"/></datamodel>
  <state id="s0">
    <onentry>
      <foreach array="outer" item="i" index="k">
        <foreach array="inner" item="j"><raise event="e"/></foreach>
      </foreach>
    </onentry>
    <transition event="e" target="done"/>
  </state>
  <final id="done"/>"#;
    let (status, machine, said) = generate(&document(r#"datamodel="ecmascript""#, body), &[]);
    assert_eq!(status, Some(0), "{said}");
    assert!(
        machine.contains(r#"if let foreachCount1 = beginForeach(1, ""#),
        "the outer loop reads its array once: {machine}"
    );
    assert!(
        machine.contains(r#"if let foreachCount2 = beginForeach(2, ""#),
        "a nested loop keeps its own array: {machine}"
    );
    assert!(
        machine.contains("bindForeach(1, at: foreachPosition1, item: ")
            && machine.contains(", index: \""),
        "the item and the index are bound to each element: {machine}"
    );
    assert!(
        machine.contains("is not an array it can walk"),
        "an array the engine cannot walk is an error.execution: {machine}"
    );
    assert!(machine.contains("break sceBlock"), "{machine}");
}

/// The data of a final state's done event is read by the engine as the state is entered and keeps its
/// type: a `<param>` and a `<content expr>` are lowered to Lua and written as JSON, a value that cannot
/// be read is an `error.execution` (a param then leaves its pair out), and an empty location names
/// nothing.
#[test]
fn the_donedata_of_a_final_is_read_by_the_engine_and_keeps_its_type() {
    let params = r#"
  <datamodel><data id="n" expr="1"/></datamodel>
  <state id="s0" initial="c"><state id="c"><transition target="d"/></state><final id="d"><donedata><param name="p" expr="n + 1"/><param name="q" location="n"/></donedata></final></state>"#;
    let (status, machine, said) = generate(&document(r#"datamodel="ecmascript""#, params), &[]);
    assert_eq!(status, Some(0), "{said}");
    assert!(
        machine.contains(r#"PayloadPair.json("p", rawJSON: paramValue)"#)
            && machine.contains(r#"PayloadPair.json("q", rawJSON: paramValue)"#),
        "each param keeps its type: {machine}"
    );
    assert!(
        machine.contains("<donedata> <param name=p> failed to evaluate"),
        "a param that cannot be read is an error.execution: {machine}"
    );

    let content = r#"
  <datamodel><data id="n" expr="1"/></datamodel>
  <state id="s0"><transition target="d"/></state>
  <final id="d"><donedata><content expr="n"/></donedata></final>"#;
    let (status, machine, said) = generate(&document(r#"datamodel="ecmascript""#, content), &[]);
    assert_eq!(status, Some(0), "{said}");
    assert!(
        machine.contains(r#"if let doneValue = evaluateJSON(""#),
        "the content keeps its type: {machine}"
    );
    assert!(
        machine.contains("<donedata> <content expr> failed to evaluate"),
        "a content that cannot be read is an error.execution: {machine}"
    );
}

/// The inline text of a `<data>` takes the ordered readings of W3C SCXML B.2: it is lowered to Lua as
/// an expression when it is one, and the string it spells, whitespace-normalized, is the last reading,
/// which the engine falls back to when the expression cannot be run. XML content is the first of
/// the readings: it is built into a DOM by the library's parser, and the string it spells is still
/// the last reading, for markup that is not a document. The name of the variable crosses as a string.
#[test]
fn the_inline_text_of_a_data_is_an_expression_or_the_string_it_spells() {
    let body = r#"
  <datamodel><data id="n">42</data><data id="words">  two   words  </data><data id="doc"><item a="1"/></data></datamodel>
  <state id="s0"><transition cond="n == 42" target="done"/></state>
  <final id="done"/>"#;
    let (status, machine, said) = generate(&document(r#"datamodel="ecmascript""#, body), &[]);
    assert_eq!(status, Some(0), "{said}");
    assert!(
        machine.contains(r#"defineData("doc", as: "_scxml_parse_xml(\"<item "#)
            && machine.contains(r#"a=\\\"1\\\"></item>\")", orText: "<item "#),
        "XML content is built into a DOM, with the string it spells as the last reading: {machine}"
    );
    assert!(
        machine.contains(r#"defineData("n", as: ""#)
            && machine.contains(r#", orText: "42", engine"#),
        "the text is lowered as an expression with its own text as the last reading: {machine}"
    );
    assert!(
        machine.contains(r#"orText: "two words""#),
        "the string is whitespace-normalized: {machine}"
    );
    assert!(
        machine.contains("orText fallback: String? = nil"),
        "the engine knows its fallback: {machine}"
    );
}

/// W3C SCXML 5.3.1: the text a `<data src>` names is read when the machine is generated and is then
/// inline text, read by the same ordered readings. The machine carries the text and not the path, so
/// it opens no file and runs where the file does not exist.
#[test]
fn the_text_a_data_src_names_is_read_when_the_machine_is_generated() {
    let body = r#"
  <datamodel><data id="list" src="file:probe.json"/><data id="note" src="probe.txt"/></datamodel>
  <state id="s0"><transition cond="list[1] == 2" target="done"/></state>
  <final id="done"/>"#;
    let (status, machine, said) = generate(
        &document(r#"datamodel="ecmascript""#, body),
        &[("probe.json", "[1, 2, 3]"), ("probe.txt", "  two   words ")],
    );
    assert_eq!(status, Some(0), "{said}");
    assert!(
        machine.contains(r#"defineData("list", as: ""#)
            && machine.contains(r#"orText: "[1, 2, 3]""#),
        "the JSON the file holds is the inline text of the variable: {machine}"
    );
    assert!(
        machine.contains(r#"orText: "two words""#),
        "the string the file holds is whitespace-normalized: {machine}"
    );
    assert!(
        !machine.contains("probe.json") && !machine.contains("probe.txt"),
        "the machine carries a path to the file it was generated from: {machine}"
    );
}

/// Under late binding a state's own `<data>` is declared on the state's first entry, before its
/// `<onentry>`, and not at document load; the data of the document itself is still declared at load.
/// The engine owns the first-entry rule, so a state entered twice binds once.
#[test]
fn late_binding_declares_a_states_data_on_its_first_entry() {
    let body = r#"
  <datamodel><data id="top" expr="1"/></datamodel>
  <state id="s0">
    <datamodel><data id="inner" expr="top + 1"/></datamodel>
    <onentry><assign location="top" expr="inner"/></onentry>
    <transition cond="top == 2" target="done"/>
  </state>
  <final id="done"/>"#;
    let (status, machine, said) = generate(
        &document(r#"datamodel="ecmascript" binding="late""#, body),
        &[],
    );
    assert_eq!(status, Some(0), "{said}");
    let load = machine
        .find("public func initializeDataModel(")
        .expect("the data of the document is declared at load");
    let load_end = machine[load..]
        .find("populateEventMetadata")
        .map(|at| load + at)
        .expect("the load ends before the event is bound");
    assert!(
        machine[load..load_end].contains(r#"defineData("top""#)
            && !machine[load..load_end].contains(r#"defineData("inner""#),
        "the document's data is declared at load and the state's is not: {machine}"
    );
    let entry = machine
        .find("engine.claimLateBindingFirstEntry(state)")
        .expect("the state's data is declared on its first entry");
    let bind = machine[entry..]
        .find(r#"defineData("inner""#)
        .map(|at| entry + at)
        .expect("the first entry declares the state's data");
    let first_onentry = machine[entry..]
        .find(r#"assignData("top""#)
        .map(|at| entry + at)
        .expect("the onentry assigns");
    assert!(
        bind < first_onentry,
        "the data is bound before the state's onentry runs: {machine}"
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

/// The event, the target and the delay of a `<send>` an expression computes are lowered to Lua and
/// read by the engine when the send runs; an `idlocation` is written to its variable by the engine
/// before any other argument is read. A name that cannot be computed is an `error.execution`, and
/// a target that names nothing is an `error.communication`.
#[test]
fn a_send_whose_event_target_and_delay_an_expression_computes_is_lowered() {
    let body = r#"
  <datamodel><data id="n" expr="1"/><data id="id"/></datamodel>
  <state id="s0">
    <onentry>
      <send idlocation="id" eventexpr="'go'" targetexpr="'#_internal'" delayexpr="'10ms'"/>
    </onentry>
    <transition event="go" target="done"/>
  </state>
  <final id="done"/>"#;
    let (status, machine, said) = generate(&document(r#"datamodel="ecmascript""#, body), &[]);
    assert_eq!(status, Some(0), "{said}");
    assert!(
        machine.matches(r#"evaluateText(""#).count() >= 3,
        "the event, the target and the delay are computed by the engine: {machine}"
    );
    assert!(
        machine.contains("let sendID = generateSendID()"),
        "{machine}"
    );
    assert!(
        machine.contains(r#"assignData("id", "\"\(sendID)\"")"#),
        "the id is written to the variable first: {machine}"
    );
    assert!(
        machine.contains("<send> eventexpr could not be evaluated to an event name"),
        "{machine}"
    );
    assert!(
        machine.contains("<send> targetexpr could not be evaluated"),
        "{machine}"
    );
    assert!(machine.contains("engine.sendToTarget("), "{machine}");
}

/// The data of a `<send>` is read by the engine and keeps its type: a `<param>`, each name of a
/// namelist and a `<content expr>` are lowered to Lua and written as JSON, a value that cannot be
/// read is an `error.execution` that leaves its pair out, and over BasicHTTP a form carries the
/// text of the value.
#[test]
fn the_data_of_a_send_is_read_by_the_engine_and_keeps_its_type() {
    let body = r#"
  <datamodel><data id="n" expr="1"/><data id="m" expr="2"/></datamodel>
  <state id="s0">
    <onentry>
      <send event="a" namelist="n m"><param name="p" expr="n + 1"/></send>
      <send event="b"><content expr="m"/></send>
      <send event="c" type="http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor" target="http://localhost/x">
        <param name="q" expr="n"/>
      </send>
    </onentry>
    <transition target="done"/>
  </state>
  <final id="done"/>"#;
    let (status, machine, said) = generate(&document(r#"datamodel="ecmascript""#, body), &[]);
    assert_eq!(status, Some(0), "{said}");
    assert!(
        machine.contains(r#"PayloadPair.json("p", rawJSON: paramValue)"#),
        "a param keeps its type: {machine}"
    );
    assert!(
        machine.contains(r#"PayloadPair.json("n", rawJSON: paramValue)"#)
            && machine.contains(r#"PayloadPair.json("m", rawJSON: paramValue)"#),
        "each name of a namelist is a pair of its own: {machine}"
    );
    assert!(
        machine.contains(r#"if let contentJSON = evaluateJSON(""#),
        "a content expression keeps its type: {machine}"
    );
    assert!(
        machine.contains(r#"appendFormParam(&sendForm, "q", .text(paramValue))"#),
        "a form carries the text of the value: {machine}"
    );
    assert!(
        machine.contains("<send> <param name=p> failed to evaluate")
            && machine.contains("<send> contentexpr failed to evaluate"),
        "a value that cannot be read is an error.execution: {machine}"
    );
}

/// A namelist names variables, and a name the document does not declare is an argument that cannot be
/// evaluated: the send is discarded with one `error.execution`, decided when the send runs as every
/// backend decides it, and the name is never resolved at build time (so no refusal is embedded that
/// the walker does not report).
#[test]
fn a_namelist_name_the_document_does_not_declare_discards_the_send() {
    let body = r#"
  <datamodel><data id="n" expr="1"/></datamodel>
  <state id="s0">
    <onentry><send event="a" namelist="n missing"/></onentry>
    <transition event="error.execution" target="done"/>
  </state>
  <final id="done"/>"#;
    let (status, machine, said) = generate(&document(r#"datamodel="ecmascript""#, body), &[]);
    assert_eq!(status, Some(0), "{said}");
    assert!(
        machine.contains("<send> namelist names 'missing', which is not declared"),
        "the send is an argument error: {machine}"
    );
    assert!(
        machine.contains(r#"PayloadPair.json("n", rawJSON: paramValue)"#),
        "a declared name is still read: {machine}"
    );
    assert!(
        !machine.contains(r#"PayloadPair.json("missing""#),
        "an undeclared name is never read: {machine}"
    );
    assert!(
        !machine.contains("is not declared by this document"),
        "no build-time refusal is embedded for the name: {machine}"
    );
}

/// W3C SCXML 6.2: a `typeexpr` is read when the send runs, with the current value of what it names,
/// and the machine refuses a value that is not a processor it has; the route is the one the element
/// was lowered for, and a value naming BasicHTTP is refused and not delivered as an SCXML event.
#[test]
fn a_typeexpr_is_checked_when_the_send_runs() {
    let body = r#"
  <datamodel><data id="kind"/></datamodel>
  <state id="s0">
    <onentry>
      <assign location="kind" expr="'http://www.w3.org/TR/scxml/#SCXMLEventProcessor'"/>
      <send event="e" typeexpr="kind"/>
    </onentry>
    <transition event="e" target="done"/>
  </state>
  <final id="done"/>"#;
    let (status, machine, said) = generate(&document(r#"datamodel="ecmascript""#, body), &[]);
    assert_eq!(status, Some(0), "{said}");
    assert!(
        machine.contains(r#"evaluateText("kind")"#) && machine.contains("isSupportedSendType("),
        "the type is read from the engine and checked: {machine}"
    );
    assert!(
        machine.contains("typeexpr names a processor this platform does not support")
            && machine.contains("typeexpr names the BasicHTTP processor")
            && machine.contains("typeexpr could not be evaluated"),
        "each way the type can be refused is its own argument error: {machine}"
    );
}

/// W3C SCXML 5.8: a `<script src>` that cannot be downloaded rejects the document. A rejected
/// document is a successful run that wrote a stub, as it is for every other backend, so there is no
/// machine, and the constant is what says the document was rejected.
#[test]
fn a_document_whose_script_cannot_be_downloaded_is_rejected_with_a_stub() {
    let body = r#"<script src="/nonexistent_swift_test_script.js"/><state id="s0"/>"#;
    let (status, machine, said) = generate(&document("", body), &[]);
    assert_eq!(
        status,
        Some(0),
        "a rejected document is not an error: {said}"
    );
    assert!(
        machine.contains("Document rejected")
            && machine.contains("public let sceDocumentRejectedProbe = true"),
        "the stub says the document was rejected: {machine}"
    );
    assert!(
        !machine.contains("class "),
        "a rejected document has no machine: {machine}"
    );
}

/// A BasicHTTP send's body is the form its `<param>`s make, so a `<content expr>` there has nowhere
/// to go and is refused by name.
#[test]
fn a_content_expression_over_basic_http_is_refused_by_name() {
    let body = r#"<datamodel><data id="n" expr="1"/></datamodel>
  <state id="s0"><onentry><send event="x" type="http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor" target="http://localhost/x"><content expr="n"/></send></onentry></state>"#;
    let (status, machine, said) = generate(&document(r#"datamodel="ecmascript""#, body), &[]);
    assert_eq!(
        status,
        Some(7),
        "a content expression over BasicHTTP must be refused, not written: {said}"
    );
    assert!(
        machine.is_empty(),
        "a content expression over BasicHTTP wrote a machine anyway"
    );
    assert!(
        said.contains("<send") && said.contains("no Swift codegen path yet"),
        "it is refused, but not by name: {said}"
    );
}

/// W3C SCXML B.2: any ECMAScript expression is a value, a function among them. The initialiser is
/// lowered to the Lua function the session holds under the variable's name, and a later expression
/// calls it.
#[test]
fn a_function_expression_is_an_initialiser_like_any_other() {
    let body = r#"
  <datamodel><data id="inc" expr="function(n) { return n + 1; }"/></datamodel>
  <state id="s0"><transition cond="inc(2) == 3" target="done"/></state>
  <final id="done"/>"#;
    let (status, machine, said) = generate(&document(r#"datamodel="ecmascript""#, body), &[]);
    assert_eq!(status, Some(0), "{said}");
    assert!(
        machine.contains(r#"defineData("inc", as: "function("#),
        "the initialiser is the lowered function: {machine}"
    );
}
