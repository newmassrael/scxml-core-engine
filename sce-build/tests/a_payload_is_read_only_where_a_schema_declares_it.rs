// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// `datamodel="sce-static"`: `_event.data` is the payload of the event a transition
// is on, read through the schema an import declares for that event
// (docs/SCE_ACCEPTED_SUBSET.md §2.15). A read of it anywhere else names no value of
// the machine. It was refused in a guard and in a `<param>` alone: in any other
// expression it was judged a value of no type, which fits every place, and the
// generated machine held the read as the author wrote it, which does not compile.

use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::{tempdir, TempDir};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// A statechart that declares the payload of `m.asked` by the schema `Ask`, whose event is
/// `declared`, and reads it in `body`, the content of a transition on `m.asked`, or in
/// `entry`, the content of a state's entry.
fn machine(declared: &str, body: &str, entry: &str) -> (TempDir, PathBuf) {
    let dir = tempdir().expect("tempdir");
    std::fs::copy(
        repo_root().join("sce-build/tests/fixtures/static_datamodel/algorithm_day_run.scxml"),
        dir.path().join("algorithm_day_run.scxml"),
    )
    .expect("the algorithm of the fixtures");
    std::fs::write(
        dir.path().join("ask.scxml"),
        format!(
            r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0"
       sce:kind="event-schema" name="ask" sce:event-name="{declared}">
  <datamodel><data id="n" sce:type="uint8" sce:direction="in"/></datamodel>
</scxml>
"##
        ),
    )
    .expect("write the schema");
    let path = dir.path().join("probe.scxml");
    // A statechart keeps no import it does not call.
    let algorithm = if body.contains("Range(") {
        r#"<sce:import kind="algorithm" src="algorithm_day_run.scxml" as="Range"/>"#
    } else {
        ""
    };
    std::fs::write(
        &path,
        format!(
            r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static" name="probe">
  <sce:import kind="event-schema" src="ask.scxml" as="Ask"/>
  {algorithm}
  <datamodel>
    <data id="shown" sce:type="list&lt;uint8&gt;" sce:capacity="8"/>
    <data id="n" sce:type="uint8" expr="3"/>
    <data id="kept" sce:type="record:Ask"><sce:set name="n" expr="0"/></data>
  </datamodel>
  <state id="s">
    <transition event="{declared}" target="t"><assign location="n" expr="_event.data.n"/></transition>
    <transition event="m.asked">{body}</transition>
  </state>
  <state id="t">
    <onentry>{entry}</onentry>
  </state>
</scxml>
"##
        ),
    )
    .expect("write probe");
    (dir, path)
}

fn check(path: &Path, backend: Option<&str>) -> (bool, String) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sce-codegen"));
    command
        .arg("--workspace-root")
        .arg(repo_root())
        .args(["--error-format=json", "check"]);
    if let Some(backend) = backend {
        command.args(["-l", backend]);
    }
    let out = command.arg(path).output().expect("invoke sce-codegen");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout),
    )
}

/// Every place of a transition's content that reads an expression: each is accepted where the
/// schema declares the event, and refused where it declares another.
const READS: [(&str, &str); 7] = [
    (
        "a call that returns a list",
        r#"<assign location="shown" expr="Range(_event.data.n, 4)"/>"#,
    ),
    (
        "an assignment",
        r#"<assign location="n" expr="_event.data.n"/>"#,
    ),
    (
        "a field of a record",
        r#"<assign location="kept.n" expr="_event.data.n"/>"#,
    ),
    (
        "an <if>",
        r#"<if cond="_event.data.n &gt; 3"><assign location="n" expr="0"/></if>"#,
    ),
    (
        "an element of a list",
        r#"<sce:append target="shown" expr="_event.data.n"/>"#,
    ),
    ("a <log>", r#"<log expr="_event.data.n"/>"#),
    (
        "a record taken whole",
        r#"<assign location="kept" expr="_event.data"/>"#,
    ),
];

#[test]
fn a_payload_is_read_where_the_schema_declares_the_event() {
    for (what, body) in READS {
        let (_dir, path) = machine("m.asked", body, "");
        for backend in [None, Some("rust")] {
            let (ok, out) = check(&path, backend);
            assert!(ok, "{what}, {backend:?}:\n{out}");
        }
    }
}

#[test]
fn a_payload_is_not_read_where_the_schema_declares_another_event() {
    for (what, body) in READS {
        let (_dir, path) = machine("m.other", body, "");
        for backend in [None, Some("rust")] {
            let (ok, out) = check(&path, backend);
            assert!(!ok, "{what}, {backend:?}: accepted");
            assert!(
                out.contains("scxml/static-datamodel-rule"),
                "{what}, {backend:?}:\n{out}"
            );
        }
    }
}

/// An entry has no event, whatever the transition before it read.
#[test]
fn an_entry_reads_no_payload() {
    let (_dir, path) = machine(
        "m.asked",
        "",
        r#"<assign location="n" expr="_event.data.n"/>"#,
    );
    let (ok, out) = check(&path, None);
    assert!(!ok, "accepted");
    assert!(out.contains("no payload is in scope"), "{out}");
}
