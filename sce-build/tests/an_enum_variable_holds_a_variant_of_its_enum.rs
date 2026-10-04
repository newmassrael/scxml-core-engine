// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// `datamodel="sce-static"`: a variable declared `enum:<alias>`
// (docs/SCE_ACCEPTED_SUBSET.md §2.15).
//
// The expression typer declines to claim a type for an enum value, so nothing
// but this pass stops `mode === 3`, `mode + 1` or `count = mode` from being
// accepted and written out, to fail in the generated code's own compiler where
// the author never sees the document. Each refusal here is one an enum value
// would otherwise have walked past, and the accepted cases are what the
// refusals leave: stored in a variable of its own enum, compared to a value of
// it, chosen between by a conditional, logged.
//
// The last cases hold what a saved state binds an enum variable to: the set of
// its variants, by name and not by position.

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

/// An enum document with the given `variants` (`name`, `value`) and root
/// attributes.
fn enum_doc(name: &str, root_attributes: &str, variants: &[(&str, u32)]) -> String {
    let variants: String = variants
        .iter()
        .map(|(variant, value)| format!("<sce:variant name=\"{variant}\" value=\"{value}\"/>"))
        .collect();
    format!(
        r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="enum" name="{name}" sce:underlying-type="uint8" {root_attributes}>
  <datamodel>
    <data id="variants">{variants}</data>
  </datamodel>
</scxml>
"##
    )
}

fn mode_enum() -> String {
    enum_doc("mode", "", &[("fast", 0), ("slow", 1)])
}

fn unit_enum() -> String {
    enum_doc("unit", "", &[("metric", 0), ("imperial", 1)])
}

/// A statechart importing `mode.scxml` as `Mode` and `unit.scxml` as `Unit`,
/// with the given `<data>` elements and the body of its one state.
fn machine(data: &str, body: &str) -> String {
    format!(
        r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static">
  <sce:import kind="enum" src="mode.scxml" as="Mode"/>
  <sce:import kind="enum" src="unit.scxml" as="Unit"/>
  <datamodel>
    <data id="count" sce:type="uint32" expr="0"/>
    {data}
  </datamodel>
  <state id="s">
    {body}
  </state>
</scxml>
"##
    )
}

const MODE: &str = r#"<data id="mode" sce:type="enum:Mode" expr="Mode.fast"/>"#;

/// `sce-codegen <args…> probe.scxml` over `doc`, with the two enum documents
/// beside it, as `(succeeded, stdout + stderr)`.
fn run(args: &[&str], doc: &str) -> (bool, String) {
    let (ok, out, _) = run_with(args, doc, &mode_enum());
    (ok, out)
}

/// [`run`] with `mode` as the text of the `Mode` enum document; also the
/// directory it ran in, which a `generate` writes its output to.
fn run_with(args: &[&str], doc: &str, mode: &str) -> (bool, String, tempfile::TempDir) {
    let dir = tempdir().expect("tempdir");
    std::fs::write(dir.path().join("mode.scxml"), mode).expect("write mode");
    std::fs::write(dir.path().join("unit.scxml"), unit_enum()).expect("write unit");
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
        dir,
    )
}

fn refused(data: &str, body: &str, because: &str) {
    let (ok, out) = run(&["check"], &machine(data, body));
    assert!(!ok, "expected a refusal ({because}), got:\n{out}");
    assert!(
        out.contains(because),
        "the refusal must say {because:?}:\n{out}"
    );
}

#[test]
fn an_enum_value_is_accepted_stored_compared_chosen_between_and_logged() {
    let body = r#"
    <onentry><log label="mode" expr="mode"/></onentry>
    <transition event="a" cond="mode === Mode.fast" type="internal">
      <assign location="mode" expr="Mode.slow"/>
      <assign location="count" expr="count + 1"/>
    </transition>
    <transition event="b" cond="mode !== other" type="internal">
      <assign location="mode" expr="other"/>
    </transition>
    <transition event="c" type="internal">
      <assign location="other" expr="mode === Mode.slow ? Mode.fast : Mode.slow"/>
    </transition>"#;
    let data = format!(r#"{MODE}<data id="other" sce:type="enum:Mode" expr="Mode.slow"/>"#);
    for args in [
        &["check"][..],
        &["check", "-l", "rust"],
        &["check", "-l", "kotlin"],
    ] {
        let (ok, out) = run(args, &machine(&data, body));
        assert!(ok, "{args:?}: an enum value used as an enum value:\n{out}");
    }
}

#[test]
fn a_variable_that_starts_at_no_variant_of_its_enum_is_refused() {
    refused(
        r#"<data id="mode" sce:type="enum:Mode" expr="1"/>"#,
        "",
        "not of the enum `Mode`",
    );
    refused(
        r#"<data id="mode" sce:type="enum:Mode" expr="Unit.metric"/>"#,
        "",
        "not of the enum `Mode`",
    );
    // No value at all is the rule every `<data>` of this data model keeps.
    refused(
        r#"<data id="mode" sce:type="enum:Mode"/>"#,
        "",
        "every <data> declares its initial value with expr",
    );
}

#[test]
fn a_variant_the_enum_does_not_declare_is_refused() {
    refused(
        r#"<data id="mode" sce:type="enum:Mode" expr="Mode.medium"/>"#,
        "",
        "medium",
    );
}

#[test]
fn an_enum_value_is_compared_only_with_a_value_of_its_own_enum() {
    for cond in [
        "mode === 1",
        "1 === mode",
        "mode === Unit.metric",
        "mode === count",
    ] {
        refused(
            MODE,
            &format!(r#"<transition event="e" cond="{cond}" type="internal"/>"#),
            "a comparison of an enum value with anything but a value of the same enum",
        );
    }
}

#[test]
fn an_enum_value_has_no_order_and_no_arithmetic() {
    for cond in [
        "mode &lt; Mode.slow",
        "mode + 1 &gt; 0",
        "!mode",
        "mode &amp;&amp; true",
    ] {
        refused(
            MODE,
            &format!(r#"<transition event="e" cond="{cond}" type="internal"/>"#),
            "an enum value used as an operand or an argument",
        );
    }
}

#[test]
fn an_enum_value_is_not_stored_in_a_number_nor_a_number_in_an_enum() {
    refused(
        MODE,
        r#"<transition event="e" type="internal"><assign location="count" expr="mode"/></transition>"#,
        "a value of the enum `Mode` where a number or a bool is expected",
    );
    refused(
        MODE,
        r#"<transition event="e" type="internal"><assign location="mode" expr="count"/></transition>"#,
        "not of the enum `Mode`",
    );
    refused(
        MODE,
        r#"<transition event="e" type="internal"><assign location="mode" expr="Unit.metric"/></transition>"#,
        "not of the enum `Mode`",
    );
}

#[test]
fn an_enum_value_is_carried_to_a_host_as_the_name_its_enum_declares() {
    // A variable, a variant and a conditional of two values of one enum, as a
    // `<send>`'s param: the name travels as a string, which every backend spells
    // alike.
    let body = r#"<transition event="e" type="internal"><send event="out">
      <param name="m" expr="mode"/>
      <param name="v" expr="Mode.slow"/>
      <param name="n" expr="count === 0 ? Mode.fast : mode"/>
    </send></transition>"#;
    for args in [
        &["check"][..],
        &["check", "-l", "rust"],
        &["check", "-l", "kotlin"],
        &["check", "-l", "go"],
        &["check", "-l", "python"],
        &["check", "-l", "cpp"],
        &["check", "-l", "c11"],
    ] {
        let (ok, out) = run(args, &machine(MODE, body));
        assert!(ok, "{args:?}: an enum value as a param:\n{out}");
    }
}

#[test]
fn a_number_chosen_against_an_enum_value_is_no_param() {
    refused(
        MODE,
        r#"<transition event="e" type="internal"><send event="out"><param name="m" expr="count === 0 ? Mode.fast : 1"/></send></transition>"#,
        "a conditional whose branches are not both values of one enum",
    );
}

#[test]
fn a_conditional_whose_branches_disagree_is_not_a_value_of_one_enum() {
    refused(
        MODE,
        r#"<transition event="e" type="internal"><assign location="mode" expr="count === 0 ? Mode.fast : 1"/></transition>"#,
        "a conditional whose branches are not both values of one enum",
    );
}

#[test]
fn an_enum_that_admits_values_no_variant_names_is_refused() {
    let open = enum_doc(
        "mode",
        r#"sce:strict-variants="false""#,
        &[("fast", 0), ("slow", 1)],
    );
    let (ok, out, _) = run_with(&["check"], &machine(MODE, ""), &open);
    assert!(!ok, "an open set has no type to be held in:\n{out}");
    assert!(
        out.contains("a variable holds a closed enum"),
        "the refusal says why:\n{out}"
    );
}

/// The `SHAPE` of the Rust machine generated from `doc` with `mode` as the
/// `Mode` enum document.
fn shape_of(doc: &str, mode: &str) -> String {
    let out_dir = tempdir().expect("tempdir");
    let out_arg = out_dir.path().to_str().expect("utf-8 path").to_string();
    let (ok, out, _) = run_with(&["generate", "-l", "rust", "-o", &out_arg], doc, mode);
    assert!(ok, "the machine generates:\n{out}");
    let generated =
        std::fs::read_to_string(out_dir.path().join("probe_sm.rs")).expect("the generated machine");
    let line = generated
        .lines()
        .find(|l| l.contains("const SHAPE: &'static str = \""))
        .expect("a saved-state shape");
    line.split('"').nth(1).expect("the shape text").to_string()
}

#[test]
fn a_saved_state_is_bound_to_the_variants_by_name_and_not_by_position() {
    let doc = machine(MODE, "");
    let as_declared = shape_of(&doc, &mode_enum());
    let reordered = shape_of(&doc, &enum_doc("mode", "", &[("slow", 1), ("fast", 0)]));
    assert_eq!(
        as_declared, reordered,
        "a saved state holds the declared name, so the order variants are declared in is not part of it"
    );
    // A renumbered variant is the wire's concern, not the machine's: the
    // machine holds the name.
    let renumbered = shape_of(&doc, &enum_doc("mode", "", &[("fast", 5), ("slow", 9)]));
    assert_eq!(
        as_declared, renumbered,
        "the value a variant carries is not saved"
    );

    let renamed = shape_of(&doc, &enum_doc("mode", "", &[("fast", 0), ("steady", 1)]));
    assert_ne!(
        as_declared, renamed,
        "a variant renamed leaves a saved value no variant of the enum"
    );
    let grown = shape_of(
        &doc,
        &enum_doc("mode", "", &[("fast", 0), ("slow", 1), ("idle", 2)]),
    );
    assert_ne!(as_declared, grown, "a variant added changes the type");
}
