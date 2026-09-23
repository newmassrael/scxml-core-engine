// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A manifest's cause is placed where its author wrote it, as a refusal is.
//!
//! # What was wrong
//!
//! `check` and `generate` explain `needs_script_engine` and
//! `needs_host_processor` with causes, each of which promises "the same
//! `{file, line, col}` shape a diagnostic carries, so tooling anchors a
//! degradation exactly as it anchors a rejection". The causes carried the
//! model's own label instead — the file's basename, where every diagnostic
//! carries the path the caller named — and rows of the text after XInclude
//! expansion. Measured 2026-09-24: `check docs/included.scxml` placed a guard
//! written in an included fragment at row 8 of `included.scxml`, the
//! including file's closing tag.
//!
//! # What is held
//!
//! A cause in the document carries the path the caller named. A cause in a
//! spliced fragment is placed in that fragment, at its own row, and under
//! the label a refusal written in a fragment beside it carries — the anchor
//! is whatever a diagnostic's is, not a spelling this test chose.

use std::path::{Path, PathBuf};
use std::process::Command;

fn codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

/// A guard the ECMAScript datamodel cannot lower natively: a
/// `transition-guard` cause on row 7.
const GUARDED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" initial="a">
  <datamodel>
    <data id="count" expr="0"/>
  </datamodel>
  <state id="a">
    <transition event="go" cond="count + 1 === 2" target="b"/>
  </state>
  <final id="b"/>
</scxml>
"#;

/// A `<send>` of a type this build has no processor for: a host-processor
/// cause on row 5.
const HOSTED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="a">
  <state id="a">
    <onentry>
      <send type="x-probe-processor" event="ping"/>
    </onentry>
  </state>
</scxml>
"#;

/// The guarded state, spliced in from `fragment.xml`.
const INCLUDED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:xi="http://www.w3.org/2001/XInclude" version="1.0" datamodel="ecmascript" initial="a">
  <datamodel>
    <data id="count" expr="0"/>
  </datamodel>
  <xi:include href="fragment.xml"/>
  <final id="b"/>
</scxml>
"#;

/// The fragment: its root is a wrapper, and its children are spliced. The
/// guard is on row 3.
const FRAGMENT: &str = r#"<fragment xmlns="http://www.w3.org/2005/07/scxml">
  <state id="a">
    <transition event="go" cond="count + 1 === 2" target="b"/>
  </state>
</fragment>
"#;

/// The same shape with a target no state answers — a refusal in a
/// fragment, whose label is the one a fragment's cause must carry.
const BROKEN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:xi="http://www.w3.org/2001/XInclude" version="1.0" initial="a">
  <xi:include href="broken_fragment.xml"/>
  <final id="b"/>
</scxml>
"#;

const BROKEN_FRAGMENT: &str = r#"<fragment xmlns="http://www.w3.org/2005/07/scxml">
  <state id="a">
    <transition event="go" target="nowhere"/>
  </state>
</fragment>
"#;

/// Every document under `docs/`, so each is named by a path that is not
/// its basename.
fn fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let docs = dir.path().join("docs");
    std::fs::create_dir(&docs).expect("docs");
    for (name, text) in [
        ("guarded.scxml", GUARDED),
        ("hosted.scxml", HOSTED),
        ("included.scxml", INCLUDED),
        ("fragment.xml", FRAGMENT),
        ("broken.scxml", BROKEN),
        ("broken_fragment.xml", BROKEN_FRAGMENT),
    ] {
        std::fs::write(docs.join(name), text).expect("write document");
    }
    dir
}

/// `sce-codegen <args>` run from `dir`.
fn run(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new(codegen_bin())
        .current_dir(dir)
        .args(args)
        .output()
        .expect("run sce-codegen")
}

/// The manifest line a successful run prints.
fn manifest(output: &std::process::Output) -> serde_json::Value {
    assert!(
        output.status.success(),
        "expected success: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout
        .lines()
        .find(|l| l.starts_with('{'))
        .unwrap_or_else(|| panic!("no manifest line in {stdout:?}"));
    serde_json::from_str(line).expect("manifest JSON")
}

/// The location of the one cause of `kind` in `list` of `manifest`.
fn cause_at(manifest: &serde_json::Value, list: &str, kind: &str) -> serde_json::Value {
    manifest[list]
        .as_array()
        .unwrap_or_else(|| panic!("no {list} in {manifest}"))
        .iter()
        .find(|cause| cause["kind"] == kind)
        .unwrap_or_else(|| panic!("no {kind} cause in {manifest}"))["location"]
        .clone()
}

#[test]
fn a_cause_in_the_document_carries_the_path_its_caller_named() {
    let dir = fixture();
    let guarded = manifest(&run(
        dir.path(),
        &["check", "docs/guarded.scxml", "-l", "cpp"],
    ));
    assert_eq!(
        cause_at(&guarded, "script_engine_causes", "transition-guard"),
        serde_json::json!({"file": "docs/guarded.scxml", "line": 7, "col": 5}),
    );
    let hosted = manifest(&run(
        dir.path(),
        &["check", "docs/hosted.scxml", "-l", "cpp"],
    ));
    let at = cause_at(&hosted, "host_processor_causes", "send-type");
    assert_eq!(at["file"], "docs/hosted.scxml", "{at}");
    assert_eq!(at["line"], 5, "{at}");
}

#[test]
fn a_cause_in_a_fragment_is_placed_where_a_refusal_there_is() {
    let dir = fixture();
    let refused = run(
        dir.path(),
        &[
            "--error-format=json",
            "check",
            "docs/broken.scxml",
            "-l",
            "cpp",
        ],
    );
    assert!(
        !refused.status.success(),
        "the broken fragment must be refused"
    );
    let stderr = String::from_utf8_lossy(&refused.stderr);
    let record: serde_json::Value = serde_json::from_str(
        stderr
            .lines()
            .find(|l| l.starts_with('{'))
            .unwrap_or_else(|| panic!("no diagnostic in {stderr:?}")),
    )
    .expect("diagnostic JSON");
    let refusal_file = record["location"]["file"]
        .as_str()
        .unwrap_or_else(|| panic!("no location.file in {record}"));
    assert!(
        refusal_file.ends_with("broken_fragment.xml"),
        "the refusal must name its fragment: {record}"
    );
    let fragment_label = refusal_file.replace("broken_fragment.xml", "fragment.xml");

    // `check` and `generate` project the causes at different sites.
    let out = dir.path().join("out");
    std::fs::create_dir(&out).expect("output directory");
    let out = out.to_str().expect("utf-8 path");
    for args in [
        vec!["check", "docs/included.scxml", "-l", "cpp"],
        vec!["generate", "docs/included.scxml", "-o", out, "-l", "cpp"],
    ] {
        let at = cause_at(
            &manifest(&run(dir.path(), &args)),
            "script_engine_causes",
            "transition-guard",
        );
        assert_eq!(
            at,
            serde_json::json!({"file": fragment_label, "line": 3, "col": 5}),
            "{}",
            args[0]
        );
    }
}
