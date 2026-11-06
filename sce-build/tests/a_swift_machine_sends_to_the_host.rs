// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The Swift backend lowers a `<send>` to a processor the host serves (§scxml-6.2.5) of a
//! `sce-static` statechart, and with it the Mesh send of SCE_MESH.md §19: a `<send target="#peer">`
//! is lowered, as on every backend whose router is the host's, to a send of type `sce:mesh`. The
//! machine hands the whole send to the handler the host registered: the event, the target
//! uninterpreted, the `<param>`s as the form, the event's data computed once, at once or when the
//! delay the document writes is over. It does not also deliver the event locally.
//!
//! `StaticDatamodelTests` holds what the engine does with the request; this file holds the text
//! the generator writes and the line it refuses by name.

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

/// What `sce-codegen generate -l swift` made of `text`, written as `host_probe.scxml`: the exit
/// status, the text of every file written by name, and what it said.
fn generate(text: &str) -> (Option<i32>, Vec<(String, String)>, String) {
    let dir = tempdir().expect("tempdir");
    std::fs::write(dir.path().join("host_probe.scxml"), text).expect("write the document");
    let out_dir = tempdir().expect("tempdir");
    let out = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(["generate", "-l", "swift"])
        .arg(dir.path().join("host_probe.scxml"))
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

/// A document of `datamodel` whose one transition runs `send`, with a variable `n` to read.
fn document(send: &str, datamodel: &str) -> String {
    let data = if datamodel == "sce-static" {
        r#"<datamodel><data id="n" sce:type="uint32" expr="7" sce:direction="out"/></datamodel>"#
    } else {
        ""
    };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="{datamodel}" name="host_probe">
  {data}
  <state id="idle">
    <transition event="go" type="internal">
      {send}
    </transition>
  </state>
</scxml>
"#
    )
}

fn machine(files: &[(String, String)]) -> &str {
    &files
        .iter()
        .find(|(name, _)| name == "host_probe_sm.swift")
        .expect("the machine is a file of the run")
        .1
}

/// `written` holds each of `needles`, or the test names the one it lacks and shows what was
/// written.
fn holds_each(written: &str, needles: &[&str]) {
    for needle in needles {
        assert!(
            written.contains(needle),
            "the generated Swift holds `{needle}`:\n{written}"
        );
    }
}

/// A send to a peer is the request the document wrote, handed to the engine, which hands it to the
/// router the host registered: the target is the peer as written, the form is what the `<param>`s
/// spell, and the data is the pairs a local delivery would carry.
#[test]
fn a_send_to_a_peer_is_handed_to_the_host_as_the_request_the_document_wrote() {
    let send = r##"<send target="#server" event="ping"><param name="n" expr="n"/></send>"##;
    let (status, files, said) = generate(&document(send, "sce-static"));
    assert_eq!(status, Some(0), "a send to a peer is written: {said}");
    holds_each(
        machine(&files),
        &[
            "var sendForm: [String: [String]] = [:]",
            "appendFormParam(&sendForm, \"n\", sceValue)",
            "var sendParts: [PayloadPair] = []",
            "let sendEventData = payloadJSON(sendParts)",
            "let sendRequest = HostSendRequest(",
            "processorType: \"sce:mesh\",",
            "eventName: \"ping\",",
            "target: \"#server\",",
            "params: sendForm,",
            "eventData: sendEventData,",
            "invokeID: engine.currentEventInvokeID",
            "engine.performHostSend(sendRequest)",
        ],
    );
}

/// The whole of the send is the host's: the event is not also taken for a local one.
#[test]
fn a_send_to_a_peer_is_not_also_delivered_locally() {
    let send = r##"<send target="#server" event="ping"/>"##;
    let (status, files, said) = generate(&document(send, "sce-static"));
    assert_eq!(status, Some(0), "a send to a peer is written: {said}");
    let code = machine(&files);
    assert!(
        code.contains("engine.performHostSend(sendRequest)"),
        "{code}"
    );
    assert!(
        !code.contains("externalQueue") && !code.contains("engine.raiseExternal"),
        "the act is the host's alone:\n{code}"
    );
}

/// §scxml-6.2.4: a delay is a property of the send, not of the processor it names. It goes to the
/// scheduler, which performs the act when the wait is over and lets `<cancel>` reach it.
#[test]
fn a_delayed_send_to_a_peer_is_handed_to_the_scheduler() {
    let send = r##"<send target="#server" event="ping" delay="2s"/>"##;
    let (status, files, said) = generate(&document(send, "sce-static"));
    assert_eq!(
        status,
        Some(0),
        "a delayed send to a peer is written: {said}"
    );
    holds_each(
        machine(&files),
        &["engine.scheduleHostSend(delay: Duration.milliseconds(2000), request: sendRequest, sender: self.sessionID)"],
    );
}

/// A `<content>` the document writes is the request's content.
#[test]
fn a_content_the_document_writes_is_the_requests_content() {
    let send = r##"<send target="#server" event="ping"><content>hello</content></send>"##;
    let (status, files, said) = generate(&document(send, "sce-static"));
    assert_eq!(status, Some(0), "a send with a content is written: {said}");
    holds_each(machine(&files), &["content: \"hello\","]);
}

/// A `<content expr>` is one value read from the machine's fields when the send is made. It is the
/// event's data as a local delivery would carry it, and the request's content as the text it spells.
#[test]
fn a_content_an_expression_computes_is_read_once_and_handed_on_as_text() {
    let send = r##"<send target="#server" event="ping"><content expr="n"/></send>"##;
    let (status, files, said) = generate(&document(send, "sce-static"));
    assert_eq!(status, Some(0), "a computed content is written: {said}");
    holds_each(
        machine(&files),
        &[
            "let contentValue: WireValue =",
            "let sendEventData = contentValue.json",
            "content: contentValue.formText,",
        ],
    );
}

/// A send over another data model is not the static lowering's: it is named in a refusal and not
/// written wrong, as the event would otherwise also be taken for a local one.
#[test]
fn a_send_to_a_peer_of_another_data_model_is_refused_by_name() {
    let plain = r##"<send target="#server" event="ping"/>"##;
    for datamodel in ["null", "ecmascript"] {
        let (status, _, said) = generate(&document(plain, datamodel));
        assert_eq!(
            status,
            Some(7),
            "a `{datamodel}` send to a peer is refused: {said}"
        );
        assert!(
            said.contains("served by the host, which only a `sce-static` document sends"),
            "the refusal of a `{datamodel}` document names the host: {said}"
        );
    }
}
