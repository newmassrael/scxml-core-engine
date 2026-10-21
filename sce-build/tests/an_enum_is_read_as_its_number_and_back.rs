// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A value of an enum is read as the number its variant declares,
//! `<alias>.toNumber(value)`, and a number back as the variant that declares it,
//! `<alias>.fromNumber(number)`, which fails for a number no variant declares
//! (docs/adr/0047), in a `sce-static` statechart and in an algorithm.
//!
//! # What was wrong
//!
//! Measured 2026-10-21: a statechart that kept `_event.data.status`, a value of an
//! enum, in an `int64` was refused, rightly, as "a value of the enum
//! `MilestoneStatus` where a number or a bool is expected", and there was nothing it
//! could write instead: no document could read the number an enum's variant
//! declares, which ADR 0042 tells an application to keep an enum in a shared list
//! by. `scenarios/static_enum_number.json` holds the pair on every engine that runs
//! the model; these cases hold the rule and what each backend lowers it to.

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

/// The documents the fixture machine imports, beside it.
const SIBLINGS: [&str; 3] = [
    "schema_item_move.scxml",
    "schema_status_number.scxml",
    "enum_item_status.scxml",
];

/// The fixture machine's text, with `from` replaced by `to` when given.
fn machine(replace: Option<(&str, &str)>) -> String {
    let text = std::fs::read_to_string(fixtures().join("static_enum_number.scxml"))
        .expect("the fixture machine");
    match replace {
        Some((from, to)) => {
            assert!(text.contains(from), "the fixture no longer holds `{from}`");
            text.replacen(from, to, 1)
        }
        None => text,
    }
}

/// `sce-codegen <args…> <file>` in a directory holding the fixture's siblings and
/// `documents`, `file` being one of `documents`. Returns `(exit_ok, stdout + stderr)`.
fn run(args: &[&str], documents: &[(&str, &str)], file: &str) -> (bool, String) {
    let dir = tempdir().expect("tempdir");
    for name in SIBLINGS {
        std::fs::copy(fixtures().join(name), dir.path().join(name)).expect("copy a sibling");
    }
    for (name, text) in documents {
        std::fs::write(dir.path().join(name), text).expect("write a document");
    }
    let out = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(args)
        .arg(dir.path().join(file))
        .output()
        .expect("invoke sce-codegen");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout),
    )
}

/// The `check` arguments of each of the six generated backends.
fn every_backend() -> Vec<Vec<&'static str>> {
    vec![
        vec!["check", "-l", "rust"],
        vec!["check", "-l", "kotlin"],
        vec!["check", "-l", "python"],
        vec!["check", "-l", "c"],
        vec!["check", "-l", "cpp"],
        vec![
            "check",
            "-l",
            "go",
            "--go-module-prefix",
            "example.test/generated",
        ],
    ]
}

/// What `generate -l <language>` writes for `file` among `documents`, every file of
/// it in one text.
fn generated(language: &str, documents: &[(&str, &str)], file: &str) -> String {
    let dir = tempdir().expect("tempdir");
    for name in SIBLINGS {
        std::fs::copy(fixtures().join(name), dir.path().join(name)).expect("copy a sibling");
    }
    for (name, text) in documents {
        std::fs::write(dir.path().join(name), text).expect("write a document");
    }
    let out_dir = tempdir().expect("tempdir");
    let out = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .args(["generate"])
        .arg(dir.path().join(file))
        .args([
            "-l",
            language,
            "--go-module-prefix",
            "example.test/generated",
        ])
        .arg("-o")
        .arg(out_dir.path())
        .output()
        .expect("invoke sce-codegen");
    assert!(
        out.status.success(),
        "generate -l {language} {file}:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let mut text = String::new();
    let mut pending = vec![out_dir.path().to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).expect("the output directory") {
            let path = entry.expect("an entry").path();
            if path.is_dir() {
                pending.push(path);
            } else {
                text.push_str(&std::fs::read_to_string(&path).unwrap_or_default());
            }
        }
    }
    text
}

/// The machine is refused for Rust, naming `reason`.
fn refused(machine_text: &str, reason: &str) {
    let (ok, out) = run(
        &["check", "-l", "rust"],
        &[("probe.scxml", machine_text)],
        "probe.scxml",
    );
    assert!(!ok, "expected a refusal naming `{reason}`:\n{out}");
    assert!(out.contains(reason), "expected `{reason}` in:\n{out}");
}

#[test]
fn every_backend_lowers_the_machine_that_reads_an_enum_as_its_number() {
    let text = machine(None);
    for args in every_backend() {
        let (ok, out) = run(&args, &[("probe.scxml", &text)], "probe.scxml");
        assert!(ok, "{args:?} must lower the machine:\n{out}");
    }
}

/// Each backend lowers the two calls to the accessors the machine's own type for
/// the enum declares, by the names and shapes of the enum document's, and wraps the
/// reading of a number in its runtime's `found` where the two shapes differ.
#[test]
fn each_backend_lowers_the_conversions_to_the_enums_accessors() {
    let text = machine(None);
    let documents = [("static_enum_number.scxml", text.as_str())];
    for (language, fragments) in [
        (
            "rust",
            &[
                "::to_underlying(",
                "sce_forge_runtime::algorithm::found(",
                "::from_underlying(",
            ][..],
        ),
        (
            "kotlin",
            &[
                "::toUnderlying)(",
                "com.sce.forge.runtime.SceChecked.found(",
                ".fromUnderlying(",
            ][..],
        ),
        (
            "cpp",
            &[
                "ToUnderlying(",
                "SCE::Forge::Checked::found(",
                "FromUnderlying(",
            ][..],
        ),
        (
            "go",
            &[".ToUnderlying(", "scealgorithm.Found(", "FromUnderlying("][..],
        ),
        (
            "python",
            &[
                ".to_underlying(",
                "sce_algorithm.found(",
                ".from_underlying(",
            ][..],
        ),
        (
            "c11",
            &[
                "enum_item_status_number(",
                "enum_item_status_from_number_take(&sce_failure_, enum_item_status_from_number(",
            ][..],
        ),
    ] {
        let out = generated(language, &documents, "static_enum_number.scxml");
        for fragment in fragments {
            assert!(
                out.contains(fragment),
                "{language}: the generated machine holds `{fragment}`:\n{out}"
            );
        }
    }
}

#[test]
fn the_interpreter_reads_the_table_of_the_enum() {
    let dir = tempdir().expect("tempdir");
    for name in SIBLINGS {
        std::fs::copy(fixtures().join(name), dir.path().join(name)).expect("copy a sibling");
    }
    std::fs::write(dir.path().join("probe.scxml"), machine(None)).expect("write the machine");
    let out_dir = tempdir().expect("tempdir");
    let out = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("lower")
        .arg(dir.path().join("probe.scxml"))
        .arg("-o")
        .arg(out_dir.path())
        .output()
        .expect("invoke sce-codegen");
    assert!(
        out.status.success(),
        "lower must accept the machine:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let lowered = std::fs::read_dir(out_dir.path())
        .expect("the output directory")
        .map(|e| std::fs::read_to_string(e.expect("an entry").path()).unwrap_or_default())
        .collect::<String>();
    assert!(
        lowered.contains("SceStatic.number.bind(null,")
            && lowered.contains("SceStatic.variant.bind(null,"),
        "the two calls read the library's table of the enum:\n{lowered}"
    );
}

#[test]
fn to_number_takes_a_value_of_its_own_enum() {
    refused(
        &machine(Some((
            "Status.toNumber(_event.data.status)",
            "Status.toNumber(3)",
        ))),
        "a value that is not of the enum `Status` handed to `Status.toNumber`",
    );
}

#[test]
fn from_number_takes_a_number() {
    refused(
        &machine(Some((
            "Status.fromNumber(_event.data.number)",
            "Status.fromNumber(Status.open)",
        ))),
        "handed to `Status.fromNumber`, which takes a number",
    );
}

#[test]
fn what_each_answers_stands_where_its_kind_does() {
    // `fromNumber` is a value of the enum, which no number holds ...
    refused(
        &machine(Some((
            r#"<assign location="number" expr="Status.toNumber(_event.data.status)"/>"#,
            r#"<assign location="number" expr="Status.fromNumber(7)"/>"#,
        ))),
        "a value of the enum `Status` where a number or a bool is expected",
    );
    // ... and `toNumber` a number, which no variable of the enum holds.
    refused(
        &machine(Some((
            r#"<assign location="status" expr="Status.fromNumber(_event.data.number)"/>"#,
            r#"<assign location="status" expr="Status.toNumber(Status.done)"/>"#,
        ))),
        "a value that is not of the enum `Status` for a variable declared enum:Status",
    );
}

/// A machine that is being built has no `error.execution` to raise, so a reading
/// of a number, which may fail, is no initial value.
#[test]
fn a_variable_does_not_start_at_a_reading_that_may_fail() {
    refused(
        &machine(Some((
            r#"expr="Status.open" sce:direction="out""#,
            r#"expr="Status.fromNumber(2)" sce:direction="out""#,
        ))),
        "an initial value that can overflow or fail",
    );
}

/// An algorithm that imports the enum `Status` and is named `name`, whose signature
/// is `params` and returns `ret`, `may_fail` saying whether it declares so, and
/// whose body returns `value`.
fn algorithm(name: &str, params: &str, ret: &str, may_fail: bool, value: &str) -> String {
    let may_fail = if may_fail { r#" may-fail="true""# } else { "" };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="{name}" version="1.0">
  <sce:import kind="enum" src="enum_item_status.scxml" as="Status"/>
  <sce:signature>
    {params}
    <sce:return type="{ret}"{may_fail}/>
  </sce:signature>
  <sce:body>
    <sce:return expr="{value}"/>
  </sce:body>
</scxml>
"#
    )
}

#[test]
fn an_algorithm_reads_an_enum_as_its_number_and_back_on_every_backend() {
    let to = algorithm(
        "status_number",
        r#"<sce:param name="s" type="enum:Status"/>"#,
        "uint8",
        false,
        "Status.toNumber(s)",
    );
    let from = algorithm(
        "number_status",
        r#"<sce:param name="n" type="uint8"/>"#,
        "enum:Status",
        true,
        "Status.fromNumber(n)",
    );
    for (file, text) in [("status_number.scxml", &to), ("number_status.scxml", &from)] {
        for args in every_backend() {
            let (ok, out) = run(&args, &[(file, text)], file);
            assert!(ok, "{file} {args:?} must lower the algorithm:\n{out}");
        }
    }
    // An algorithm reaches the enum document's own accessors, and C reads a number
    // through the family it declares beside the enum.
    let documents = [("number_status.scxml", from.as_str())];
    let rust = generated("rust", &documents, "number_status.scxml");
    assert!(
        rust.contains("sce_forge_runtime::algorithm::found(")
            && rust.contains("::from_underlying("),
        "{rust}"
    );
    let c = generated("c11", &documents, "number_status.scxml");
    assert!(
        c.contains("SCE_FOUND_ENUM_ITEM_STATUS_FROM_NUMBER")
            && c.contains("enum_item_status_from_number_take(&sce_failure_,"),
        "{c}"
    );
}

/// A number no variant declares is a failure, which only an algorithm that declares
/// `may-fail` passes on.
#[test]
fn an_algorithm_reads_a_number_only_where_it_may_fail() {
    let text = algorithm(
        "number_status",
        r#"<sce:param name="n" type="uint8"/>"#,
        "enum:Status",
        false,
        "Status.fromNumber(n)",
    );
    let (ok, out) = run(
        &["check", "-l", "rust"],
        &[("number_status.scxml", &text)],
        "number_status.scxml",
    );
    assert!(!ok, "expected a refusal:\n{out}");
    assert!(
        out.contains("may-fail"),
        "the refusal names may-fail:\n{out}"
    );
}
