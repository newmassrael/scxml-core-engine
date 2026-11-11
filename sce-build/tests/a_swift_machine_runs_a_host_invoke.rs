// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The Swift backend lowers an `<invoke>` a host runs (§scxml-6.4.1) of a `sce-static`
//! statechart whose request carries nothing: the invoke is deferred to the end of the
//! macrostep like any other, handed to the invoker the host registered for its type under
//! the id the document wrote, and cancelled when its state exits. What the host
//! completes it with is a typed record the generator writes beside the machine, with an
//! invoker protocol and the functions that register and complete one.
//!
//! The `<param>`s are the fields of the request record when the invoke types one: each is
//! evaluated once, kept as the text the host reads and as the typed pair the request's JSON
//! is written from, and the adapter rebuilds the record from that text. A
//! `_sce_deadline_ms` among them is the engine's, which arms it and does not hand it on, and
//! the policy names the invokes a host runs so the engine refuses a `done.invoke` that did
//! not come through the completion door. What else the request carries (a `src`, a
//! `namelist`, a `<content>`) is refused by name and not written wrong, until the slice
//! that lowers it.
//!
//! `StaticDatamodelTests` replays the machines against a reference host; this file holds
//! the text the generator writes and the line it refuses by name.

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

fn fixtures() -> PathBuf {
    repo_root().join("sce-build/tests/fixtures/static_datamodel")
}

/// What `sce-codegen generate -l swift` made of `machine`, a document of the fixture
/// directory or one of `extra` (name and text, written beside the fixtures): the exit
/// status, the text of every file written by name, and what it said.
fn generate(machine: &str, extra: &[(&str, &str)]) -> (Option<i32>, Vec<(String, String)>, String) {
    let dir = tempdir().expect("tempdir");
    for entry in std::fs::read_dir(fixtures()).expect("the fixture directory") {
        let path = entry.expect("an entry").path();
        if path.is_file() {
            let name = path.file_name().expect("a file name").to_owned();
            std::fs::copy(&path, dir.path().join(name)).expect("copy a fixture");
        }
    }
    for (name, text) in extra {
        std::fs::write(dir.path().join(name), text).expect("write a document");
    }
    let out_dir = tempdir().expect("tempdir");
    let out = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(["generate", "-l", "swift"])
        .arg(dir.path().join(machine))
        .arg("-o")
        .arg(out_dir.path())
        .output()
        .expect("invoke sce-codegen");
    let mut files = Vec::new();
    for entry in std::fs::read_dir(out_dir.path()).expect("the output directory") {
        let path = entry.expect("an entry").path();
        if path.is_file() {
            let name = path
                .file_name()
                .expect("a file name")
                .to_string_lossy()
                .into_owned();
            files.push((name, std::fs::read_to_string(&path).unwrap_or_default()));
        }
    }
    files.sort();
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    (out.status.code(), files, said)
}

/// `written` holds each of `needles`, or the test names the one it lacks and shows
/// what was written.
fn holds_each(written: &str, needles: &[&str]) {
    for needle in needles {
        assert!(
            written.contains(needle),
            "the generated Swift holds `{needle}`:\n{written}"
        );
    }
}

fn clock_machine() -> String {
    let (status, files, said) = generate("static_host_clock.scxml", &[]);
    assert_eq!(status, Some(0), "static_host_clock is written: {said}");
    files
        .into_iter()
        .find(|(name, _)| name == "static_host_clock_sm.swift")
        .expect("the machine is a file of the run")
        .1
}

/// The invoke is started under the id the document wrote and handed to the engine, which
/// hands it to the invoker the host registered; a state that exits cancels it.
#[test]
fn a_host_invoke_is_started_under_the_documents_id_and_cancelled_on_exit() {
    let code = clock_machine();
    holds_each(
        &code,
        &[
            "private func sceStartHostInvokeNow(engine: Engine<StaticHostClockPolicy>)",
            "processorType: \"sce-host-clock-now\",",
            "invokeID: \"now\",",
            "if !engine.performHostInvoke(request) {",
            "engine.cancelHostInvoke(processorType: \"sce-host-clock-now\", invokeID: \"now\")",
            "deferInvoke(&pendingInvokes, PendingInvoke(",
        ],
    );
}

/// The host answers with the record of the document's `sce:result`: a struct of its fields
/// with the JSON `done.invoke.<id>` carries, a protocol to implement, and the functions
/// that register it and complete a start.
#[test]
fn a_host_invoke_has_a_typed_result_an_invoker_and_a_completion() {
    let code = clock_machine();
    holds_each(
        &code,
        &[
            "public struct StaticHostClockNowResult: Equatable, Sendable {",
            "public var wire: String {",
            "payloadJSON(",
            "public protocol StaticHostClockSceHostClockNowInvoker: AnyObject {",
            "func startNow(token: UInt64) -> StaticHostClockNowResult?",
            "func cancelNow(token: UInt64)",
            "public func registerStaticHostClockSceHostClockNowInvoker(on engine: Engine<StaticHostClockPolicy>, invoker: StaticHostClockSceHostClockNowInvoker) throws(HostRegistrationError)",
            "try engine.registerInvoker(\"sce-host-clock-now\")",
            "public func completeStaticHostClockNow(on engine: Engine<StaticHostClockPolicy>, token: UInt64, result: StaticHostClockNowResult) -> Bool",
            "engine.completeHostInvoke(processorType: \"sce-host-clock-now\", invokeID: \"now\", token: token, doneData: result.wire)",
        ],
    );
}

/// The `<param>`s of a typed request are collected where the invocation starts, and the
/// adapter hands the host the record they make, read back from their text.
#[test]
fn a_typed_request_is_collected_at_the_start_and_rebuilt_for_the_host() {
    let (status, files, said) = generate("static_host_kv.scxml", &[]);
    assert_eq!(status, Some(0), "static_host_kv is written: {said}");
    let code = &files
        .iter()
        .find(|(name, _)| name == "static_host_kv_sm.swift")
        .expect("the machine is a file of the run")
        .1;
    holds_each(
        code,
        &[
            "var hostParams: [String: [String]] = [:]",
            "var hostParts: [PayloadPair] = []",
            "appendFormParam(&hostParams, \"key\", sceValue)",
            "params: hostParams,",
            "eventData: hostParts.isEmpty ? \"\" : payloadJSON(hostParts),",
            "public struct StaticHostKvPutRequest: Equatable, Sendable {",
            "func startPut(_ request: StaticHostKvPutRequest, token: UInt64) -> StaticHostKvPutResult?",
            "let typed = StaticHostKvPutRequest(key: start.requestString(\"key\"), value: start.requestString(\"value\"))",
            "if let result = invoker.startPut(typed, token: start.token) {",
        ],
    );
}

/// The policy of a machine that runs host invokes names them, and a deadline is collected
/// like any other param and left to the engine to take out.
#[test]
fn a_policy_names_its_host_invokes_and_a_deadline_is_left_to_the_engine() {
    let (status, files, said) = generate("static_host_http.scxml", &[]);
    assert_eq!(status, Some(0), "static_host_http is written: {said}");
    let code = &files
        .iter()
        .find(|(name, _)| name == "static_host_http_sm.swift")
        .expect("the machine is a file of the run")
        .1;
    holds_each(
        code,
        &[
            "public var hostInvokeIDs: [String] { [\"fetch\"] }",
            "appendFormParam(&hostParams, \"_sce_deadline_ms\", sceValue)",
        ],
    );
    // A machine that runs none names none.
    let (status, files, said) = generate("static_invoke.scxml", &[]);
    assert_eq!(status, Some(0), "static_invoke is written: {said}");
    let plain = &files
        .iter()
        .find(|(name, _)| name == "static_invoke_sm.swift")
        .expect("the machine is a file of the run")
        .1;
    assert!(
        !plain.contains("hostInvokeIDs"),
        "no host invoke, no ids:\n{plain}"
    );
}

/// A machine that runs no host invoke writes none of it.
#[test]
fn a_machine_without_a_host_invoke_writes_no_invoker() {
    let (status, files, said) = generate("static_invoke.scxml", &[]);
    assert_eq!(status, Some(0), "static_invoke is written: {said}");
    let code = &files
        .iter()
        .find(|(name, _)| name == "static_invoke_sm.swift")
        .expect("the machine is a file of the run")
        .1;
    assert!(
        !code.contains("performHostInvoke") && !code.contains("registerInvoker"),
        "no host invoke, no invoker:\n{code}"
    );
}

/// §scxml-6.4.1: an `<invoke>` whose `type` no processor implements and no host declared is
/// written, and what it does is one error.execution when the invoke would have started: it
/// rides the deferral every other invoke does, so a state that exits first raises nothing.
#[test]
fn an_invoke_of_a_type_nobody_runs_is_deferred_and_raises_error_execution() {
    let text = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="idle"
       datamodel="sce-static" name="unsupported_type">
  <state id="idle">
    <invoke type="no-such-type" id="x"/>
    <transition event="error.execution" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#;
    let (status, files, said) = generate(
        "unsupported_type.scxml",
        &[("unsupported_type.scxml", text)],
    );
    assert_eq!(status, Some(0), "the document is written: {said}");
    let code = &files
        .iter()
        .find(|(name, _)| name == "unsupported_type_sm.swift")
        .expect("the machine is a file of the run")
        .1;
    assert!(
        code.contains("deferInvoke(&pendingInvokes"),
        "the invoke is deferred to the end of the macrostep:\n{code}"
    );
    assert!(
        code.contains("names a processor this platform does not implement"),
        "the execution raises error.execution:\n{code}"
    );
    assert!(
        !code.contains("performHostInvoke") && !code.contains("registerInvoker"),
        "no host runs it:\n{code}"
    );
}

/// An invoke's request a script data model computes is read by the engine, as the invocation
/// starts, and one that cannot be read is `error.execution` (W3C SCXML 6.4), not a request the
/// build refuses: the same machine was refused by name before the engine read a host invoke's
/// request in Swift (`eaec0e0a6e`).
#[test]
fn a_srcexpr_a_script_engine_computes_is_read_by_the_engine_when_the_invoke_starts() {
    // A Mesh request of a script data model whose peer a `srcexpr` computes: `sce-static`
    // validation refuses an `idlocation` and a `namelist` on a host-run invoke before the Swift
    // backend is asked, and a `srcexpr` is a string of the machine's fields only for a
    // `sce-static` document, so what reaches the backend is the script data model's.
    let with_src = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="idle"
       datamodel="ecmascript" name="host_computed">
  <datamodel><data id="x" expr="'#peer'"/></datamodel>
  <state id="idle"><transition event="go" target="asking"/></state>
  <state id="asking">
    <invoke id="ask" type="sce:mesh-rpc" srcexpr="x">
      <param name="_mesh_event" expr="'service.request'"/>
    </invoke>
    <transition event="done.invoke.ask" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"##
    .to_string();
    let (status, files, said) =
        generate("host_computed.scxml", &[("host_computed.scxml", &with_src)]);
    assert_eq!(
        status,
        Some(0),
        "a host invoke whose srcexpr the engine reads is written: {said}"
    );
    let code = &files
        .iter()
        .find(|(name, _)| name == "host_computed_sm.swift")
        .expect("the machine is a file of the run")
        .1;
    holds_each(
        code,
        &[
            "guard let hostSrc = evaluateText(\"x\") else {",
            "raiseExecutionError(\"<invoke> srcexpr failed to evaluate\", engine: engine)",
            "src: hostSrc,",
        ],
    );
}

/// The Mesh request fixture (`statechart_static_mesh_request`): a host-run invoke whose
/// request is the untyped kind, which carries a `src` and `<param>`s.
fn mesh_request_fixture() -> String {
    std::fs::read_to_string(
        repo_root()
            .join("sce-build/tests/fixtures/host_processor/statechart_static_mesh_request.scxml"),
    )
    .expect("the Mesh request fixture")
}

/// A `src` the document writes is the request's, as written.
#[test]
fn a_src_the_document_writes_is_the_requests() {
    let written = mesh_request_fixture().replacen(r#"srcexpr="peer""#, r##"src="#motor""##, 1);
    assert!(
        written.contains(r##"src="#motor""##),
        "the fixture gained a src"
    );
    let (status, files, said) = generate(
        "statechart_static_mesh_request.scxml",
        &[("statechart_static_mesh_request.scxml", &written)],
    );
    assert_eq!(status, Some(0), "an invoke with a src is written: {said}");
    let code = &files
        .iter()
        .find(|(name, _)| name == "statechart_static_mesh_request_sm.swift")
        .expect("the machine is a file of the run")
        .1;
    holds_each(code, &["src: \"#motor\","]);
    assert!(
        !code.contains("hostSrc"),
        "a src the document writes is not computed:\n{code}"
    );
}

/// A Mesh request (SCE_MESH.md §9.5) is lowered by the build to a host-served invoke of
/// `sce:mesh-rpc`; its peer is the string an expression computes from the machine's fields
/// when the request starts, and its envelope is the `_mesh_*` params the router reads.
#[test]
fn a_mesh_request_names_its_peer_by_an_expression_read_when_it_starts() {
    let request = mesh_request_fixture();
    let (status, files, said) = generate(
        "statechart_static_mesh_request.scxml",
        &[("statechart_static_mesh_request.scxml", &request)],
    );
    assert_eq!(status, Some(0), "a Mesh request is written: {said}");
    let code = &files
        .iter()
        .find(|(name, _)| name == "statechart_static_mesh_request_sm.swift")
        .expect("the machine is a file of the run")
        .1;
    holds_each(
        code,
        &[
            "let hostSrc: String",
            "src: hostSrc,",
            "processorType: \"sce:mesh-rpc\",",
            "appendFormParam(&hostParams, \"_mesh_event\"",
            "appendFormParam(&hostParams, \"_mesh_deadline_ms\"",
            "engine.performHostInvoke(request)",
        ],
    );
}
