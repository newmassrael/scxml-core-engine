// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The Swift backend lowers a `<send>` to a processor the host serves (§scxml-6.2.5) of a
//! statechart of any data model, and with it the Mesh send of SCE_MESH.md §19: a `<send
//! target="#peer">` is lowered, as on every backend whose router is the host's, to a send of type
//! `sce:mesh`, and a `targetexpr` that names a peer once it is evaluated reaches the same act. The
//! machine hands the whole send to the handler the host registered: the event, the target
//! uninterpreted, the `<param>`s as the form, the event's data computed once, at once or when the
//! delay the document writes is over. It does not also deliver the event locally.
//!
//! `StaticDatamodelTests` and the integration drivers hold what the engine does with the request;
//! this file holds the text the generator writes.

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
    generate_with(text, &[])
}

/// [`generate`], with the `<send>` types the host declares it serves (`--host-processor`).
fn generate_serving(text: &str, serves: &[&str]) -> (Option<i32>, Vec<(String, String)>, String) {
    let declared: Vec<&str> = serves
        .iter()
        .flat_map(|served| ["--host-processor", *served])
        .collect();
    generate_with(text, &declared)
}

/// [`generate`], with `extra` arguments after the output directory.
fn generate_with(text: &str, extra: &[&str]) -> (Option<i32>, Vec<(String, String)>, String) {
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
        .args(extra)
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

/// A document of `datamodel` whose one transition runs `send`, with a variable `n` to read and,
/// under `ecmascript`, a variable `peer` that names a Mesh peer.
fn document(send: &str, datamodel: &str) -> String {
    let data = match datamodel {
        "sce-static" => {
            r#"<datamodel><data id="n" sce:type="uint32" expr="7" sce:direction="out"/></datamodel>"#
        }
        "ecmascript" => {
            r#"<datamodel><data id="n" expr="7"/><data id="peer" expr="'#hmi'"/></datamodel>"#
        }
        _ => "",
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

/// The calls by which a Swift machine delivers an event its send makes to a session of its own
/// tree, at once or after a delay. A send the host serves makes none of them.
const LOCAL_DELIVERIES: &[&str] = &[
    "engine.raiseExternalWithMeta(",
    "engine.sendToTarget(",
    "engine.scheduleEvent(",
    "engine.scheduleRoutedEvent(",
    "engine.sendNamedExternal(",
    "engine.sendNamedInternal(",
    "self.deliverToParent(",
    "EventMetadata.external(",
];

/// `code` hands the act to the host exactly once and delivers nothing locally: the request is made
/// in one place and the event is not also put on a queue of this machine, where the document would
/// see the act twice.
fn hands_the_host_the_act_once(code: &str, datamodel: &str) {
    assert_eq!(
        code.matches("let sendRequest = HostSendRequest(").count(),
        1,
        "a `{datamodel}` send makes one request:\n{code}"
    );
    assert_eq!(
        code.matches("engine.performHostSend(sendRequest)").count(),
        1,
        "a `{datamodel}` send is handed to the host once:\n{code}"
    );
    for local in LOCAL_DELIVERIES {
        assert!(
            !code.contains(local),
            "a `{datamodel}` send the host serves is not also delivered locally (`{local}`):\n{code}"
        );
    }
}

/// A send to a peer is the host's under every data model, not only `sce-static`: the build lowers it
/// to a `sce:mesh` send, and the machine hands the router the request and delivers nothing itself.
#[test]
fn a_send_to_a_peer_of_any_data_model_is_handed_to_the_host_once() {
    let plain = r##"<send target="#server" event="ping"/>"##;
    for datamodel in ["null", "ecmascript"] {
        let (status, files, said) = generate(&document(plain, datamodel));
        assert_eq!(
            status,
            Some(0),
            "a `{datamodel}` send to a peer is written: {said}"
        );
        let code = machine(&files);
        holds_each(
            code,
            &["processorType: \"sce:mesh\",", "target: \"#server\","],
        );
        hands_the_host_the_act_once(code, datamodel);
    }
}

/// §scxml-6.2.5: an `ecmascript` document's send to a type the host declared is the host's whole.
/// A `<param>` is read once by the engine, and the one value is both the form the handler reads and
/// the pair the event's data carries.
#[test]
fn an_ecmascript_send_to_a_declared_type_is_handed_to_the_host_once() {
    let send = r#"<send type="x-sce-host" event="host.ask"><param name="n" expr="n + 1"/></send>"#;
    let (status, files, said) = generate_serving(&document(send, "ecmascript"), &["x-sce-host"]);
    assert_eq!(status, Some(0), "a send to the host is written: {said}");
    let code = machine(&files);
    holds_each(
        code,
        &[
            "private func evaluateWire(_ expression: String) -> (text: String, json: String)?",
            "var sendForm: [String: [String]] = [:]",
            "var sendParts: [PayloadPair] = []",
            "if let paramWire = evaluateWire(",
            "appendFormParam(&sendForm, \"n\", .text(paramWire.text))",
            "sendParts.append(PayloadPair.json(\"n\", rawJSON: paramWire.json))",
            "let sendEventData = payloadJSON(sendParts)",
            "processorType: \"x-sce-host\",",
            "eventName: \"host.ask\",",
            "target: \"\",",
            "params: sendForm,",
            "eventData: sendEventData,",
            "invokeID: engine.currentEventInvokeID",
        ],
    );
    assert!(
        !code.contains("rawJSON: paramValue"),
        "the param is not read a second time for the event's data:\n{code}"
    );
    hands_the_host_the_act_once(code, "ecmascript");
}

/// §scxml-6.2.4: a delay an `ecmascript` document computes is a property of the send, not of the
/// processor it names: the request waits on the scheduler, where `<cancel>` reaches it.
#[test]
fn a_delayed_ecmascript_send_to_a_declared_type_is_handed_to_the_scheduler() {
    let send = r#"<send type="x-sce-host" event="host.ask" delayexpr="'2s'"/>"#;
    let (status, files, said) = generate_serving(&document(send, "ecmascript"), &["x-sce-host"]);
    assert_eq!(
        status,
        Some(0),
        "a delayed send to the host is written: {said}"
    );
    let code = machine(&files);
    holds_each(
        code,
        &["engine.scheduleHostSend(delay: Duration.milliseconds(Int64(sendDelayMs)), request: sendRequest, sender: self.sessionID)"],
    );
    assert!(
        !code.contains("engine.performHostSend(sendRequest)"),
        "a delayed send is not also performed at once:\n{code}"
    );
    for local in LOCAL_DELIVERIES {
        assert!(
            !code.contains(local),
            "a delayed send the host serves is not also delivered locally (`{local}`):\n{code}"
        );
    }
}

/// SCE_MESH.md §mesh-19: a `targetexpr` that names a peer once it is evaluated reaches the host's
/// Mesh router as a literal `#peer` does, and with none registered it is a send to a host processor
/// nobody serves (`performHostSend`'s `error.execution`). Any other value is routed as a target.
#[test]
fn a_peer_an_expression_names_is_handed_to_the_router() {
    let send = r#"<send event="ping" targetexpr="peer"><param name="n" expr="n"/></send>"#;
    let (status, files, said) = generate(&document(send, "ecmascript"));
    assert_eq!(status, Some(0), "a computed target is written: {said}");
    let code = machine(&files);
    holds_each(
        code,
        &[
            "private func evaluateWire(_ expression: String) -> (text: String, json: String)?",
            "if isMeshTarget(sendTarget) {",
            "processorType: \"sce:mesh\",",
            "target: sendTarget,",
            "appendFormParam(&sendForm, \"n\", .text(paramWire.text))",
            "params: sendForm,",
            "engine.performHostSend(sendRequest)",
        ],
    );
    assert!(
        !code.contains("declared but never registered"),
        "a peer is handed to the router, not refused before it is asked:\n{code}"
    );
    assert_eq!(
        code.matches("engine.performHostSend(sendRequest)").count(),
        1,
        "the peer is handed to the router once:\n{code}"
    );
    assert_eq!(
        code.matches("engine.sendToTarget(").count(),
        1,
        "a value that names no peer is routed as a target, once:\n{code}"
    );
    let routed = code
        .find("if isMeshTarget(sendTarget) {")
        .expect("the peer branch is written");
    let local = code
        .find("engine.sendToTarget(")
        .expect("the target branch is written");
    let otherwise = code[routed..]
        .find("} else {")
        .map(|at| routed + at)
        .expect("the peer branch has an else");
    assert!(
        routed < otherwise && otherwise < local,
        "the local route is the else of the peer branch, so one send takes one of them:\n{code}"
    );
}

/// §scxml-6.2.4 + §scxml-5.10 (test 332): the request a host is handed names the send by the id an
/// error the send causes carries, which the engine reads from the request (`performHostSend`): the
/// id the document wrote, the one the machine generates for an `idlocation` before it reads
/// anything else, or the one the build gives a send that names none. A request with no id would
/// raise an error that names no send.
#[test]
fn a_host_send_hands_the_host_the_id_its_error_names() {
    let written = r#"<send type="x-sce-host" event="host.ask" id="mine"/>"#;
    let generated = r#"<send type="x-sce-host" event="host.ask" idlocation="n"/>"#;
    let unnamed = r#"<send type="x-sce-host" event="host.ask"/>"#;
    for (send, needles) in [
        (written, &["sendID: \"mine\","][..]),
        (
            generated,
            &["let sendID = generateSendID()", "sendID: sendID,"][..],
        ),
        (unnamed, &["sendID: \"__send_"][..]),
    ] {
        let (status, files, said) =
            generate_serving(&document(send, "ecmascript"), &["x-sce-host"]);
        assert_eq!(status, Some(0), "`{send}` is written: {said}");
        let code = machine(&files);
        holds_each(code, needles);
        assert!(
            !code.contains("sendID: \"\","),
            "`{send}` hands the host a request that names no send:\n{code}"
        );
        hands_the_host_the_act_once(code, "ecmascript");
    }
}
