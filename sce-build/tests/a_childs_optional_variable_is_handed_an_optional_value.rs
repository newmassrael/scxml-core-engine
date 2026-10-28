// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A child session's variable of `optional<T>` is handed the whole optional value
//! its `<param>` computes (docs/adr/0055, step 10): a value of `T` is present,
//! `null` is absent, an optional variable of the parent is handed as it is, and a
//! value not handed is the child's own start. So a child whose optional variable
//! starts at `null` takes any of them, and one that starts at a value takes only a
//! value that is never absent; a list, a record, an enum, a byte string and an
//! optional value of the last three are handed no value.
//!
//! The judge's rules are held through `sce-codegen check`; what each backend
//! writes for the slot of such a variable (the child's `InvokeParams`) is held
//! through `sce-codegen generate`, with every backend lowering the machine
//! `static_invoke_optional.scxml`, whose scenario each backend's own driver
//! replays.
//!
//! # What was wrong
//!
//! Measured in the code on 2026-10-28: `<param name="due" expr="count"/>` into a
//! child's `optional<int64>` was refused by name ("hands no value to yet"), since
//! the child's arguments spell a name not handed as absent and a variable that may
//! be absent seemed to need a second absence inside that one.

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

/// The documents of the fixture directory a probe imports, copied beside it: the
/// enum the parent holds an optional value of, and the machine of the scenario.
const SIBLINGS: [&str; 2] = ["enum_item_status.scxml", "static_invoke_optional.scxml"];

const DATAMODEL_RULE: &str = "scxml/static-datamodel-rule";
const TYPE_MISMATCH: &str = "expression/type-mismatch";

/// `sce-codegen <args…> <file>` in a directory holding the siblings and
/// `documents`, `file` being one of them or a sibling. Returns `(exit_ok, stderr +
/// stdout, the text of every file written)`.
fn run(args: &[&str], documents: &[(&str, &str)], file: &str) -> (bool, String, String) {
    let dir = tempdir().expect("tempdir");
    for name in SIBLINGS {
        std::fs::copy(fixtures().join(name), dir.path().join(name)).expect("copy a sibling");
    }
    for (name, text) in documents {
        std::fs::write(dir.path().join(name), text).expect("write a document");
    }
    let out_dir = tempdir().expect("tempdir");
    let mut command = Command::new(sce_codegen_bin());
    command
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(args)
        .arg(dir.path().join(file));
    if matches!(args.first(), Some(&"generate")) {
        command.arg("-o").arg(out_dir.path());
    }
    let out = command.output().expect("invoke sce-codegen");
    let mut written = String::new();
    for entry in std::fs::read_dir(out_dir.path()).expect("the output directory") {
        let path = entry.expect("an entry").path();
        if path.is_file() {
            written.push_str(&std::fs::read_to_string(&path).unwrap_or_default());
            written.push('\n');
        }
    }
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout),
        written,
    )
}

/// The `generate` arguments of each backend that emits a statechart's machine.
fn every_backend() -> Vec<Vec<&'static str>> {
    sce_build::forge::codegen_matrix::emitting(sce_build::forge::model::ForgeKind::Statechart)
        .map(|language| {
            let mut args = vec!["generate", "-l", language.canonical_name()];
            if language == sce_build::generator::Language::Go {
                args.extend(["--go-module-prefix", "example.test/generated"]);
            }
            args
        })
        .collect()
}

/// A `sce-static` statechart, `probe.scxml`, that invokes `child.scxml` (with
/// `attrs` on the `<invoke>`) and hands it `params`, beside the variables an
/// optional value is handed from: an optional day, text, flag and value of
/// `Status`, and a plain number.
fn parent(attrs: &str, params: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="child_probe">
  <sce:import kind="enum" src="enum_item_status.scxml" as="Status"/>
  <datamodel>
    <data id="due" sce:type="optional&lt;int64&gt;" expr="null"/>
    <data id="title" sce:type="optional&lt;string&gt;" sce:capacity="16" expr="null"/>
    <data id="flag" sce:type="optional&lt;bool&gt;" expr="null"/>
    <data id="status" sce:type="optional&lt;enum:Status&gt;" expr="null"/>
    <data id="count" sce:type="int64" expr="0"/>
  </datamodel>
  <state id="idle">
    <invoke type="scxml" id="child" src="child.scxml"{attrs}>{params}</invoke>
    <transition event="done.invoke.child" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#
    )
}

/// A `sce-static` child document, `child.scxml`, with `imports` and the top-level
/// `<data>` `data`.
fn child(imports: &str, data: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="busy" datamodel="sce-static">
  {imports}
  <datamodel>
    {data}
  </datamodel>
  <state id="busy"><transition event="finish" target="end"/></state>
  <final id="end"/>
</scxml>
"#
    )
}

/// A child whose variables start at `null`: an optional day, text and flag.
fn child_starting_absent() -> String {
    child(
        "",
        r#"<data id="due" sce:type="optional&lt;int64&gt;" expr="null"/>
    <data id="title" sce:type="optional&lt;string&gt;" sce:capacity="16" expr="null"/>
    <data id="flag" sce:type="optional&lt;bool&gt;" expr="null"/>"#,
    )
}

/// A child whose optional day starts at 3.
fn child_starting_present() -> String {
    child(
        "",
        r#"<data id="due" sce:type="optional&lt;int64&gt;" expr="3"/>"#,
    )
}

/// `probe` (with `child` beside it as `child.scxml`) is judged fit and lowered for
/// Rust: refused by no rule of the data model or of an expression, and by no
/// lowering.
fn judged_fit(probe: &str, child: &str) {
    let (ok, out, _) = run(
        &["check", "-l", "rust"],
        &[("probe.scxml", probe), ("child.scxml", child)],
        "probe.scxml",
    );
    assert!(
        !out.contains(DATAMODEL_RULE) && !out.contains("\"code\":\"expression/"),
        "the document is judged fit:\n{out}"
    );
    assert!(ok, "Rust lowers the document:\n{out}");
}

/// `probe` (with `child` beside it) is refused with `code`, and the refusal holds
/// `needle`.
fn refused(probe: &str, child: &str, code: &str, needle: &str) {
    let (ok, out, _) = run(
        &["check", "-l", "rust"],
        &[("probe.scxml", probe), ("child.scxml", child)],
        "probe.scxml",
    );
    assert!(!ok, "expected a refusal with {code}:\n{out}");
    assert!(out.contains(code), "expected the code {code} in:\n{out}");
    assert!(out.contains(needle), "expected `{needle}` in:\n{out}");
}

// ── What the judge admits ───────────────────────────────────────────────

/// A variable of the parent that is optional is handed as it is, and `null` is an
/// absent value, to a child whose variables start at `null`.
#[test]
fn an_optional_value_and_null_are_handed_to_a_variable_that_starts_absent() {
    judged_fit(
        &parent(
            "",
            r#"<param name="due" expr="due"/>
      <param name="title" expr="title"/>
      <param name="flag" expr="null"/>"#,
        ),
        &child_starting_absent(),
    );
}

/// A value of `T` is a present value of the child's `optional<T>`, held to the
/// bound the child declares for a text.
#[test]
fn a_value_of_t_is_handed_as_a_present_value() {
    judged_fit(
        &parent(
            "",
            r#"<param name="due" expr="count"/>
      <param name="title" expr="'soon'"/>
      <param name="flag" expr="true"/>"#,
        ),
        &child_starting_absent(),
    );
}

/// A `namelist` name is the `<param>` it abbreviates.
#[test]
fn a_namelist_name_hands_an_optional_variable_as_a_param_does() {
    judged_fit(&parent(r#" namelist="due""#, ""), &child_starting_absent());
}

/// A child whose optional variable starts at a value takes a value that is never
/// absent, which a value of `T` and `x ?? d` are.
#[test]
fn a_value_that_is_never_absent_is_handed_to_a_variable_that_starts_at_a_value() {
    for param in [
        r#"<param name="due" expr="count"/>"#,
        r#"<param name="due" expr="5"/>"#,
        r#"<param name="due" expr="due ?? 0"/>"#,
    ] {
        judged_fit(&parent("", param), &child_starting_present());
    }
}

// ── What it refuses ─────────────────────────────────────────────────────

/// A value that may be absent stands for no value handed, which leaves a child
/// that starts at a value that start: refused at the `<param>`, naming the start
/// and the two ways to write a value that is never absent.
#[test]
fn a_value_that_may_be_absent_is_refused_by_a_variable_that_starts_at_a_value() {
    for param in [
        r#"<param name="due" expr="due"/>"#,
        r#"<param name="due" expr="null"/>"#,
    ] {
        refused(
            &parent("", param),
            &child_starting_present(),
            DATAMODEL_RULE,
            "starts at `3`",
        );
    }
    refused(
        &parent(r#" namelist="due""#, ""),
        &child_starting_present(),
        DATAMODEL_RULE,
        "never absent",
    );
}

/// An optional value of an enum is handed no value, as an enum is not.
#[test]
fn an_optional_enum_variable_is_handed_no_value() {
    refused(
        &parent("", r#"<param name="status" expr="status"/>"#),
        &child(
            r#"<sce:import kind="enum" src="enum_item_status.scxml" as="Status"/>"#,
            r#"<data id="status" sce:type="optional&lt;enum:Status&gt;" expr="null"/>"#,
        ),
        DATAMODEL_RULE,
        "an optional value of an enum",
    );
}

/// A value of another element is no value of the child's variable.
#[test]
fn an_optional_value_of_another_element_is_refused() {
    refused(
        &parent("", r#"<param name="due" expr="flag"/>"#),
        &child_starting_absent(),
        TYPE_MISMATCH,
        "bool",
    );
}

// ── What each backend writes ────────────────────────────────────────────

/// Every generated backend lowers the machine of the scenario: a backend that
/// refused a child's optional variable by name would be found here, and each
/// backend's own spelling is held below.
#[test]
fn every_generated_backend_lowers_the_optional_child_scenario_machine() {
    for args in every_backend() {
        let (ok, out, _) = run(&args, &[], "static_invoke_optional.scxml");
        assert!(ok, "{args:?} lowers the machine:\n{out}");
    }
}

/// What `language` writes for the machine of the scenario, children included.
fn written_for(language: &str) -> String {
    let mut args = vec!["generate", "-l", language];
    if language == "go" {
        args.extend(["--go-module-prefix", "example.test/generated"]);
    }
    let (ok, out, written) = run(&args, &[], "static_invoke_optional.scxml");
    assert!(ok, "{language} lowers the machine:\n{out}");
    written
}

/// The slot of a child's optional variable is the backend's one optional type
/// itself, so that an absent seed is a value not handed: Rust's `Option<i32>` and
/// not an `Option` of one, which a caller could not tell from a value left out.
#[test]
fn rust_holds_the_slot_of_an_optional_variable_in_the_optional_type_itself() {
    let written = written_for("rust");
    assert!(
        written.contains("pub size: Option<i32>,") && written.contains("pub label: Option<"),
        "the slot is the optional type:\n{written}"
    );
    assert!(
        !written.contains("Option<Option<"),
        "no second absence:\n{written}"
    );
    assert!(
        written.contains("params.size.is_some()"),
        "a slot that is absent is a value not handed:\n{written}"
    );
}

/// Kotlin's `Int?` is the slot, with no second `?`.
#[test]
fn kotlin_holds_the_slot_of_an_optional_variable_in_the_nullable_type_itself() {
    let written = written_for("kotlin");
    assert!(
        written.contains("var size: Int? = null"),
        "the slot is the nullable type:\n{written}"
    );
    assert!(!written.contains("Int??"), "no second absence:\n{written}");
}

/// Python's `Optional[int]` is the slot, with no second `Optional`.
#[test]
fn python_holds_the_slot_of_an_optional_variable_in_the_optional_type_itself() {
    let written = written_for("python");
    assert!(
        written.contains("size: Optional[int] = None"),
        "the slot is the optional type:\n{written}"
    );
    assert!(
        !written.contains("Optional[Optional["),
        "no second absence:\n{written}"
    );
}

/// Go's `optional.Of[int32]` is the slot, handed whole and tested by `IsPresent`,
/// and not a pointer to one.
#[test]
fn go_holds_the_slot_of_an_optional_variable_in_the_optional_type_itself() {
    let written = written_for("go");
    assert!(
        written.contains("optional.Of[int32]") && written.contains("VSize.IsPresent()"),
        "the slot is the optional type:\n{written}"
    );
    assert!(
        !written.contains("*optional.Of["),
        "no pointer to a second absence:\n{written}"
    );
}

/// C++'s `std::optional<int32_t>` is the slot, and the seed is declared of its own
/// type, not of the `value_type` a slot that wraps a value would give it.
#[test]
fn cpp_holds_the_slot_of_an_optional_variable_in_the_optional_type_itself() {
    let written = written_for("cpp");
    assert!(
        written.contains("decltype(seed.v_size) seedValue"),
        "the seed is of the slot's own type:\n{written}"
    );
    assert!(
        !written.contains("std::optional<std::optional<"),
        "no second absence:\n{written}"
    );
}

/// C11 writes the whole optional value into the child's variable: the seed is the
/// runtime's `sce_optional_int32_t`, copied as one value.
#[test]
fn c11_writes_the_whole_optional_value_into_the_child() {
    let written = written_for("c11");
    assert!(
        written.contains("sce_optional_int32_t sce_value =")
            && written.contains(".policy.v_size = sce_value;"),
        "the seed is the optional value, written whole:\n{written}"
    );
}
