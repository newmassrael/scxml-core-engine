// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// `datamodel="sce-static"`: a list variable assigned what an imported algorithm
// returns as a list (docs/SCE_ACCEPTED_SUBSET.md §2.15).
//
// A list is filled by `<sce:append>`, one element at a time; a calendar screen
// that shows a run of days wants them all at once from the algorithm that
// computes them. The assignment replaces the list, so the list's bound is the
// one thing it must still hold: an algorithm that may return more than the
// variable's `sce:capacity` is refused when the document is built, because a
// machine holds the same list wherever it runs.

use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::{tempdir, TempDir};

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// A statechart with a `list<uint8>` `shown` of the given `capacity`, a
/// `list<uint16>` `wide`, a `uint8` `n` and the given `body` in one transition,
/// beside the algorithm it imports.
fn machine(capacity: u32, body: &str) -> (TempDir, PathBuf) {
    let dir = tempdir().expect("tempdir");
    for algorithm in ["algorithm_day_run.scxml", "algorithm_day_repeat.scxml"] {
        std::fs::copy(
            repo_root()
                .join("sce-build/tests/fixtures/static_datamodel")
                .join(algorithm),
            dir.path().join(algorithm),
        )
        .expect("an algorithm of the fixtures");
    }
    // A statechart keeps no import it does not call, so each is imported by the
    // body that calls it.
    let imports: String = [
        ("Range", "algorithm_day_run.scxml"),
        ("Repeat", "algorithm_day_repeat.scxml"),
    ]
    .iter()
    .filter(|(alias, _)| body.contains(&format!("{alias}(")))
    .map(|(alias, src)| format!("<sce:import kind=\"algorithm\" src=\"{src}\" as=\"{alias}\"/>"))
    .collect();
    let path = dir.path().join("probe.scxml");
    std::fs::write(
        &path,
        format!(
            r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static">
  {imports}
  <datamodel>
    <data id="shown" sce:type="list&lt;uint8&gt;" sce:capacity="{capacity}"/>
    <data id="wide" sce:type="list&lt;uint16&gt;" sce:capacity="8"/>
    <data id="n" sce:type="uint8" expr="3"/>
  </datamodel>
  <state id="s">
    <transition event="go" type="internal">
      {body}
    </transition>
  </state>
</scxml>
"##
        ),
    )
    .expect("write probe");
    (dir, path)
}

fn run(args: &[&str], capacity: u32, body: &str) -> (bool, String) {
    let (_dir, path) = machine(capacity, body);
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

#[test]
fn a_list_is_assigned_what_an_algorithm_returns_in_every_language() {
    let body = r#"<assign location="shown" expr="Range(n, 4)"/>"#;
    for args in [
        &["check"][..],
        &["check", "-l", "rust"],
        &["check", "-l", "kotlin"],
        &["check", "-l", "cpp"],
        &[
            "check",
            "-l",
            "go",
            "--go-module-prefix",
            "example.com/generated",
        ],
        &["check", "-l", "python"],
        &["check", "-l", "c11"],
    ] {
        let (ok, out) = run(args, 8, body);
        assert!(ok, "{args:?}: a list assigned from an algorithm:\n{out}");
    }
}

#[test]
fn another_algorithm_is_taken_by_the_bound_it_declares() {
    // `Repeat` holds four: a list of eight takes it, one of three cannot.
    let body = r#"<assign location="shown" expr="Repeat(n, 2)"/>"#;
    let (ok, out) = run(&["check"], 8, body);
    assert!(ok, "a list that can hold four:\n{out}");
    let (ok, out) = run(&["check"], 3, body);
    assert!(!ok, "expected a refusal, got:\n{out}");
    assert!(out.contains("at most 4"), "the refusal must say 4:\n{out}");
}

#[test]
fn only_an_algorithm_that_declares_may_fail_is_taken() {
    // The same algorithm with `may-fail` taken off returns a plain list on the
    // backends whose lists grow and a fallible one on the rest, so a statechart has
    // no one shape to receive it in: it stays a host's to call.
    let body = r#"<assign location="shown" expr="Repeat(n, 2)"/>"#;
    let (dir, path) = machine(8, body);
    let algorithm = dir.path().join("algorithm_day_repeat.scxml");
    let text = std::fs::read_to_string(&algorithm).expect("the algorithm");
    assert!(
        text.contains(" may-fail=\"true\""),
        "the fixture declares it"
    );
    std::fs::write(&algorithm, text.replace(" may-fail=\"true\"", "")).expect("rewrite");
    let out = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(["check"])
        .arg(&path)
        .output()
        .expect("invoke sce-codegen");
    let said =
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout);
    assert!(!out.status.success(), "expected a refusal, got:\n{said}");
    // The refusal says which algorithm and which condition it fails: not that a list
    // "takes what an imported algorithm returns" when this call is exactly that.
    for needed in ["`Repeat`", "may-fail"] {
        assert!(
            said.contains(needed),
            "the refusal must name {needed:?}:\n{said}"
        );
    }
}

#[test]
fn a_list_variable_must_hold_what_the_algorithm_may_return() {
    // The algorithm returns at most 8; a list of 4 cannot be assigned it.
    let (ok, out) = run(
        &["check"],
        4,
        r#"<assign location="shown" expr="Range(n, 4)"/>"#,
    );
    assert!(!ok, "expected a refusal, got:\n{out}");
    assert!(
        out.contains("at most 8"),
        "the refusal must say how many the algorithm may return:\n{out}"
    );
}

#[test]
fn only_a_list_of_the_algorithms_element_is_assigned_it() {
    let (ok, out) = run(
        &["check"],
        8,
        r#"<assign location="wide" expr="Range(n, 4)"/>"#,
    );
    assert!(!ok, "expected a refusal, got:\n{out}");
    assert!(
        out.contains("uint8") && out.contains("uint16"),
        "the refusal must name both element types:\n{out}"
    );
}

#[test]
fn a_returned_list_is_still_no_value_anywhere_but_a_whole_list_assignment() {
    for body in [
        r#"<assign location="n" expr="Range(n, 4)"/>"#,
        r#"<assign location="n" expr="len(Range(n, 4))"/>"#,
        r#"<assign location="n" expr="Range(n, 4)[0]"/>"#,
    ] {
        let (ok, out) = run(&["check"], 8, body);
        assert!(!ok, "expected a refusal for {body}, got:\n{out}");
    }
}
