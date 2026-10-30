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
//! is written from, and the adapter rebuilds the record from that text. What else the
//! request carries (a `src`, a `namelist`, a `<content>`, a deadline) is refused by name
//! and not written wrong, until the slice that lowers it.
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
            "public func registerStaticHostClockSceHostClockNowInvoker(on engine: Engine<StaticHostClockPolicy>, invoker: StaticHostClockSceHostClockNowInvoker)",
            "engine.registerInvoker(\"sce-host-clock-now\")",
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

/// An invoke whose request is not lowered yet, and one of a type no host declared, are
/// named in a refusal and not written wrong.
#[test]
fn what_a_host_invoke_carries_that_is_not_lowered_is_refused_by_name() {
    let document = |invoke: &str| {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="host_refused">
  <state id="idle">
    {invoke}
  </state>
</scxml>
"#
        )
    };
    // The clock fixture, its first invoke given a `src`: a standard host type needs its
    // result imported, so the document that names one is the fixture and not a stub.
    let with_src = std::fs::read_to_string(fixtures().join("static_host_clock.scxml"))
        .expect("the clock fixture")
        .replacen(
            r#"<invoke type="sce-host-clock-now" id="now""#,
            r#"<invoke type="sce-host-clock-now" id="now" src="somewhere""#,
            1,
        );
    assert!(
        with_src.contains("src=\"somewhere\""),
        "the fixture gained a src"
    );
    for (text, construct) in [
        (
            document(r#"<invoke type="no-such-type" id="x"/>"#),
            "an <invoke type=\"no-such-type\"> no host declared",
        ),
        (
            with_src,
            "an <invoke type=\"sce-host-clock-now\"> with a src",
        ),
    ] {
        let (status, files, said) =
            generate("host_refused.scxml", &[("host_refused.scxml", &text)]);
        assert_eq!(
            status,
            Some(7),
            "{construct} is refused, not written: {said}"
        );
        // The message arrives as JSON, which escapes the quotes of the type.
        assert!(
            said.replace("\\\"", "\"").contains(construct),
            "the refusal names {construct}: {said}"
        );
        assert!(
            files.iter().all(|(name, _)| !name.ends_with("_sm.swift")),
            "a refused document writes no machine: {:?}",
            files.iter().map(|f| &f.0).collect::<Vec<_>>()
        );
    }
}
