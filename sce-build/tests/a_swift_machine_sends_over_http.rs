// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The Swift backend lowers a BasicHTTP `<send>` (§scxml-C-2) of a `sce-static`
//! statechart that posts at once to a URL the document writes: each `<param>` is
//! evaluated once, kept as the text a form carries, and handed with the target and the
//! event to the transport the host registered. Every other axis of an HTTP send is
//! refused by name and not written wrong.
//!
//! There is no scenario file: a scenario states what a machine's fields hold and not
//! what it sent over a wire. `StaticDatamodelTests/SendOverHttpTests.swift` holds the
//! pairs with the request the engine hands to its transport; this file holds the text
//! the generator writes and the line it refuses by name.
//!
//! # What was wrong
//!
//! Measured in the code on 2026-10-31: the Swift generator refused every `<send>` that
//! named a processor but the SCXML one, so `static_send_http` had no Swift machine,
//! while the engine already held `performHTTPSend` and the callback it dispatches to.

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

/// The request is the target and event the document writes and the form its `<param>`s
/// build, handed to the engine, which hands it to the transport. Each pair is added as
/// the text it spells, in document order.
#[test]
fn a_send_posts_the_form_its_params_build_to_the_url_it_names() {
    let (status, files, said) = generate("static_send_http.scxml", &[]);
    assert_eq!(status, Some(0), "static_send_http is written: {said}");
    let code = &files
        .iter()
        .find(|(name, _)| name == "static_send_http_sm.swift")
        .expect("the machine is a file of the run")
        .1;
    holds_each(
        code,
        &[
            "var sendForm: [String: [String]] = [:]",
            "appendFormParam(&sendForm, \"count\", sceValue)",
            "appendFormParam(&sendForm, \"twice\", sceValue)",
            "engine.performHTTPSend(",
            "target: \"http://example.invalid/hook\",",
            "eventName: \"note\",",
            "content: \"\",",
            "params: sendForm,",
        ],
    );
}

/// What an HTTP send carries is the form and not the JSON an internal send writes: the
/// pairs are not written twice.
#[test]
fn an_http_send_writes_no_json_for_its_pairs() {
    let (status, files, said) = generate("static_send_http.scxml", &[]);
    assert_eq!(status, Some(0), "static_send_http is written: {said}");
    let code = &files
        .iter()
        .find(|(name, _)| name == "static_send_http_sm.swift")
        .expect("the machine is a file of the run")
        .1;
    assert!(
        !code.contains("payloadJSON(sendParts)") && !code.contains("sendParts.append"),
        "an HTTP send builds no JSON of its pairs:\n{code}"
    );
}

/// A delay needs a scheduler of HTTP requests, and a body of another kind needs a
/// `<content>` the form does not carry: each is named in a refusal and not written wrong.
#[test]
fn an_http_send_with_a_delay_or_a_body_is_refused_by_name() {
    let document = |send: &str| {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="http_refused">
  <state id="idle">
    <transition event="go" type="internal">
      {send}
    </transition>
  </state>
</scxml>
"#
        )
    };
    let basic = "http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor";
    for (send, construct) in [
        (
            format!(
                r#"<send event="note" delay="1s" type="{basic}" target="http://example.invalid/hook"/>"#
            ),
            "a delayed BasicHTTP <send>",
        ),
        (
            format!(
                r#"<send event="note" type="{basic}" target="http://example.invalid/hook"><content>body</content></send>"#
            ),
            "a BasicHTTP <send> with <content>",
        ),
    ] {
        let text = document(&send);
        let (status, files, said) =
            generate("http_refused.scxml", &[("http_refused.scxml", &text)]);
        assert_eq!(
            status,
            Some(7),
            "{construct} is refused, not written: {said}"
        );
        assert!(
            said.contains(construct),
            "the refusal names {construct}: {said}"
        );
        assert!(
            files.iter().all(|(name, _)| !name.ends_with("_sm.swift")),
            "a refused document writes no machine: {:?}",
            files.iter().map(|f| &f.0).collect::<Vec<_>>()
        );
    }
}
