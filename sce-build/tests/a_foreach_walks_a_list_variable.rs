// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// `datamodel="sce-static"`: a `<foreach>` over a list variable
// (docs/SCE_ACCEPTED_SUBSET.md §2.15, W3C SCXML 4.6).
//
// The loop's variables are the body's own — the item typed as the list's
// element, the index a `uint32` — and the generated code declares them in
// every backend as written, so each name has to be one a backend can declare
// and none in scope can already mean. Each refusal here is one a loop would
// otherwise have walked past into a generated file that does not compile, or
// compiles and means something else.

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

/// A statechart with a `list<uint8>` `picked`, a `uint32` `total`, a `bool`
/// `flag` and the given `body` in one transition.
fn machine(body: &str) -> String {
    format!(
        r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static">
  <datamodel>
    <data id="picked" sce:type="list&lt;uint8&gt;" sce:capacity="4"/>
    <data id="total" sce:type="uint32" expr="0"/>
    <data id="flag" sce:type="bool" expr="false"/>
  </datamodel>
  <state id="s">
    <transition event="go" type="internal">
      {body}
    </transition>
  </state>
</scxml>
"##
    )
}

/// `sce-codegen <args…> probe.scxml` over `doc`, as `(succeeded, output)`.
fn run(args: &[&str], doc: &str) -> (bool, String) {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("probe.scxml");
    std::fs::write(&path, doc).expect("write probe");
    let out = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(args)
        .arg(&path)
        .output()
        .expect("invoke sce-codegen");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout),
    )
}

fn refused(body: &str, because: &str) {
    let (ok, out) = run(&["check"], &machine(body));
    assert!(!ok, "expected a refusal ({because}), got:\n{out}");
    assert!(
        out.contains(because),
        "the refusal must say {because:?}:\n{out}"
    );
}

#[test]
fn a_loop_over_a_list_is_accepted_with_an_item_an_index_and_nesting() {
    let body = r#"
      <foreach array="picked" item="v">
        <assign location="total" expr="total + v"/>
      </foreach>
      <foreach array="picked" item="v" index="i">
        <assign location="total" expr="total + v * (i + 1)"/>
        <foreach array="picked" item="w">
          <assign location="total" expr="total + w"/>
        </foreach>
      </foreach>"#;
    for args in [
        &["check"][..],
        &["check", "-l", "rust"],
        &["check", "-l", "kotlin"],
    ] {
        let (ok, out) = run(args, &machine(body));
        assert!(ok, "{args:?}: a <foreach> over a list variable:\n{out}");
    }
}

#[test]
fn a_loop_walks_a_list_variable_and_nothing_else() {
    refused(
        r#"<foreach array="total" item="v"/>"#,
        "walks a list variable",
    );
    refused(
        r#"<foreach array="nothing" item="v"/>"#,
        "walks a list variable",
    );
    refused(
        r#"<foreach array="picked" item=""/>"#,
        "names the variable that holds each element",
    );
}

#[test]
fn a_loop_variable_is_a_name_nothing_in_scope_already_means() {
    for (written, name) in [
        (r#"item="total""#, "total"),
        (r#"item="picked""#, "picked"),
        (r#"item="v" index="total""#, "total"),
        (r#"item="v" index="v""#, "v"),
    ] {
        refused(
            &format!(r#"<foreach array="picked" {written}/>"#),
            &format!("{name}"),
        );
    }
    refused(
        r#"<foreach array="picked" item="v"><foreach array="picked" item="v"/></foreach>"#,
        "which this one would hide",
    );
}

#[test]
fn a_loop_variable_is_a_name_the_generated_code_can_declare() {
    refused(
        r#"<foreach array="picked" item="a-b"/>"#,
        "a name the generated code can spell",
    );
    refused(
        r#"<foreach array="picked" item="sce_v"/>"#,
        "does not begin `sce_`",
    );
    // A keyword in one backend is a name no document can use in any.
    for keyword in ["fun", "match", "class"] {
        refused(
            &format!(r#"<foreach array="picked" item="{keyword}"/>"#),
            "no backend reserves as a keyword",
        );
    }
}

#[test]
fn the_item_is_typed_as_the_lists_element_and_belongs_to_the_body() {
    // A `uint8` is not a `bool`: the item is typed, not left to any use.
    let (ok, out) = run(
        &["check"],
        &machine(
            r#"<foreach array="picked" item="v"><assign location="flag" expr="v"/></foreach>"#,
        ),
    );
    assert!(!ok, "a uint8 item stored in a bool:\n{out}");
    // Outside the loop the name means nothing.
    let (ok, out) = run(
        &["check"],
        &machine(
            r#"<foreach array="picked" item="v"/>
      <assign location="total" expr="total + v"/>"#,
        ),
    );
    assert!(!ok, "the item read after the loop:\n{out}");
}
