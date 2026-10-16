// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! `datamodel="sce-static"` reads `_event.data` through the schema its event
//! declares (docs/SCE_ACCEPTED_SUBSET.md §2.15). The data of an event with none
//! (the failure code of an `error.*` event) is text this model has no type for,
//! so a guard that reads it is refused where it is written, rather than accepted
//! and written into the generated machine as the text the author wrote, which
//! names no value there.

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

/// `sce-codegen check <doc>` with JSON diagnostics: whether it succeeded, and
/// everything it wrote.
fn check(doc: &str) -> (bool, String) {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("probe.scxml");
    std::fs::write(&path, doc).expect("write probe");
    let out = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .arg("check")
        .arg(&path)
        .output()
        .expect("invoke sce-codegen");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout),
    )
}

/// A statechart with one transition on `error.invoke.x`, guarded by `cond`.
fn doc(cond: &str) -> String {
    format!(
        r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="a" datamodel="sce-static" name="probe">
  <datamodel>
    <data id="n" sce:type="uint32" expr="0" sce:direction="out"/>
  </datamodel>
  <state id="a">
    <transition event="error.invoke.x"{cond} target="a">
      <assign location="n" expr="n + 1"/>
    </transition>
  </state>
</scxml>
"#
    )
}

#[test]
fn a_transition_on_an_event_with_no_schema_is_accepted_without_a_guard_on_its_data() {
    let (ok, text) = check(&doc(""));
    assert!(
        ok,
        "a transition on an event with no schema was refused: {text}"
    );
}

#[test]
fn a_guard_reading_the_data_of_an_event_with_no_schema_is_refused() {
    let (ok, text) = check(&doc(r#" cond="_event.data === 'deadline'""#));
    assert!(
        !ok,
        "a guard reading text this data model cannot type was accepted: {text}"
    );
    assert!(text.contains("scxml/static-datamodel-rule"), "{text}");
}
