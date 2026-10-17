// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// `datamodel="sce-static"`: an expression reads the payload of an event, `_event.data.<field>`,
// and no other member of `_event` (docs/SCE_ACCEPTED_SUBSET.md §2.15).
//
// The generated machine is native code with no `_event` of its own to read the metadata of an
// event from (its origin, its name, its type), so an expression that names one was written
// into the code as it stood: `check` accepted it, `generate` wrote `if _event.origin == …` in
// every language, and the compiler of the generated language was what refused it, as an
// unknown name nobody had declared. It is refused where it is written, under the data model
// that has no such read, and says what it reads instead. Under `ecmascript` the same
// expression is the script engine's to evaluate and is not refused here.

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

/// A statechart under `datamodel` whose one transition has the guard `cond` and the
/// assignment `expr`.
fn machine(datamodel: &str, cond: &str, expr: &str) -> String {
    format!(
        r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="{datamodel}">
  <datamodel>
    <data id="n" sce:type="uint32" expr="0"/>
    <data id="word" sce:type="string" sce:capacity="16" expr="''"/>
  </datamodel>
  <state id="s">
    <transition event="go" cond="{cond}" type="internal">
      <assign location="n" expr="{expr}"/>
    </transition>
  </state>
</scxml>
"##
    )
}

fn check(doc: &str) -> (bool, String) {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("probe.scxml");
    std::fs::write(&path, doc).expect("write probe");
    let out = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(["check", "-l", "rust"])
        .arg(&path)
        .output()
        .expect("invoke sce-codegen");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout),
    )
}

#[test]
fn a_guard_that_reads_the_origin_of_an_event_is_refused() {
    let (ok, out) = check(&machine(
        "sce-static",
        "_event.origin === 'mesh://alice'",
        "n + 1",
    ));
    assert!(!ok, "expected a refusal, got:\n{out}");
    assert!(
        out.contains("_event.origin") && out.contains("payload of an event"),
        "the refusal must name what was read and what is read instead:\n{out}"
    );
}

#[test]
fn an_assignment_that_reads_other_metadata_is_refused() {
    for member in ["name", "type", "sendid", "invokeid", "origintype"] {
        let (ok, out) = check(&machine("sce-static", "true", &format!("_event.{member}")));
        assert!(!ok, "_event.{member} was taken:\n{out}");
        assert!(
            out.contains(&format!("_event.{member}")),
            "the refusal must name _event.{member}:\n{out}"
        );
    }
}

#[test]
fn the_payload_of_an_event_and_a_text_that_names_it_are_not_refused() {
    // The payload is what a machine of this data model reads (the document has no schema
    // for `go`, so it is an untyped guard here only for the shape of the probe), and a
    // string that says `_event.origin` reads nothing.
    let (ok, out) = check(&machine("sce-static", "true", "n + 1"));
    assert!(ok, "a machine that reads nothing of an event:\n{out}");
    let doc =
        machine("sce-static", "true", "n + 1").replace("expr=\"''\"", "expr=\"'_event.origin'\"");
    let (ok, out) = check(&doc);
    assert!(ok, "a literal that spells it reads nothing:\n{out}");
}

#[test]
fn the_script_engine_data_model_reads_the_metadata_as_it_always_did() {
    let doc = r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s" datamodel="ecmascript">
  <datamodel>
    <data id="n" expr="0"/>
  </datamodel>
  <state id="s">
    <transition event="go" cond="_event.name === 'go'" type="internal">
      <assign location="n" expr="1"/>
    </transition>
  </state>
</scxml>
"##;
    let (ok, out) = check(doc);
    assert!(ok, "the script engine evaluates it:\n{out}");
}
