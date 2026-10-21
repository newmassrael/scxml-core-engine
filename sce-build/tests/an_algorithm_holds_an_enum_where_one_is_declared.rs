// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! An algorithm's body holds a value of an enum where its signature or a
//! declaration says a value of that enum stands, and a number never does — the
//! rule a `sce-static` statechart's expressions keep (docs/SCE_ACCEPTED_SUBSET.md
//! §2.15, "Enum variables"), held by `forge::static_enum::check_algorithm`.
//!
//! # What was wrong
//!
//! The expression typer types an enum value `Unknown`, so a slot of a number
//! admitted it and an enum slot admitted a number. Measured 2026-10-21, an
//! algorithm that returned its `enum:Status` parameter where its signature
//! returns `int64`, or its `int64` parameter where it returns `enum:Status`,
//! passed `check` on all six backends, and the Rust it generated
//! (`fn status_number(s: Status) -> i64 { return s; }`) did not compile.

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::forge::codegen_matrix::language_wire_name;
use sce_build::generator::Language;

fn codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

/// The enum every probe imports: declared values that are not the variants'
/// positions.
const STATUS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="enum" name="probe_status" sce:underlying-type="uint8">
  <datamodel>
    <data id="variants">
      <sce:variant name="open" value="2"/>
      <sce:variant name="done" value="7"/>
    </data>
  </datamodel>
</scxml>
"#;

/// A document to check, and the file it is written to.
struct Probe {
    file: String,
    text: String,
}

/// An algorithm named `name` whose signature is `params` and returns `ret`, whose
/// body is `body`, written to `<name>.scxml`. `imports` are written before the
/// signature, beside the import of the enum.
fn algorithm(name: &str, imports: &str, params: &str, ret: &str, body: &str) -> Probe {
    Probe {
        file: format!("{name}.scxml"),
        text: format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="{name}" version="1.0">
  <sce:import kind="enum" src="probe_status.scxml" as="Status"/>
  {imports}
  <sce:signature>
    {params}
    <sce:return type="{ret}"/>
  </sce:signature>
  <sce:body>
    {body}
  </sce:body>
</scxml>
"#
        ),
    }
}

/// `sce-codegen check <probe> -l <wire>` in JSON mode, in a directory that holds
/// the enum, `probe` and `beside`: the exit status and what it wrote.
fn check(beside: &[Probe], probe: &Probe, wire: &str) -> (bool, String) {
    let dir = tempfile::tempdir().expect("tempdir");
    write(dir.path(), "probe_status.scxml", STATUS);
    for document in beside.iter().chain(std::iter::once(probe)) {
        write(dir.path(), &document.file, &document.text);
    }
    let output = Command::new(codegen_bin())
        .current_dir(dir.path())
        .args([
            "--error-format=json",
            "check",
            &probe.file,
            "-l",
            wire,
            "--go-module-prefix",
            "example.com/probe",
        ])
        .output()
        .expect("run sce-codegen");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stderr).into_owned()
            + &String::from_utf8_lossy(&output.stdout),
    )
}

fn write(dir: &Path, name: &str, text: &str) {
    std::fs::write(dir.join(name), text).expect("write a document");
}

/// `probe`, with `beside` written next to it, is refused on every backend as
/// `expression/unsupported-construct`, naming `reason`.
fn refused_everywhere(beside: &[Probe], probe: &Probe, reason: &str) {
    let mut wrong = Vec::new();
    for &lang in Language::ALL {
        let wire = language_wire_name(lang);
        let (ok, out) = check(beside, probe, wire);
        if ok {
            wrong.push(format!("[{wire}] accepted:\n{out}"));
        } else if !out.contains("\"code\":\"expression/unsupported-construct\"")
            || !out.contains(reason)
        {
            wrong.push(format!("[{wire}] not refused naming `{reason}`:\n{out}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// `probe`, with `beside` written next to it, is accepted on every backend.
fn accepted_everywhere(beside: &[Probe], probe: &Probe) {
    let mut refused = Vec::new();
    for &lang in Language::ALL {
        let wire = language_wire_name(lang);
        let (ok, out) = check(beside, probe, wire);
        if !ok {
            refused.push(format!("[{wire}]:\n{out}"));
        }
    }
    assert!(refused.is_empty(), "{}", refused.join("\n"));
}

#[test]
fn an_enum_parameter_is_not_returned_as_a_number() {
    refused_everywhere(
        &[],
        &algorithm(
            "status_number",
            "",
            r#"<sce:param name="s" type="enum:Status"/>"#,
            "int64",
            r#"<sce:return expr="s"/>"#,
        ),
        "a value of the enum `Status` where a number or a bool is expected",
    );
}

#[test]
fn a_number_is_not_returned_as_an_enum() {
    refused_everywhere(
        &[],
        &algorithm(
            "number_status",
            "",
            r#"<sce:param name="n" type="int64"/>"#,
            "enum:Status",
            r#"<sce:return expr="n"/>"#,
        ),
        "a value that is not of the enum `Status`",
    );
}

#[test]
fn a_local_holds_a_value_of_its_own_kind() {
    refused_everywhere(
        &[],
        &algorithm(
            "local_number",
            "",
            r#"<sce:param name="s" type="enum:Status"/>"#,
            "uint8",
            r#"<sce:var name="n" type="uint8" init="Status.open"/>
    <sce:return expr="n"/>"#,
        ),
        "a value of the enum `Status` where a number or a bool is expected",
    );
    refused_everywhere(
        &[],
        &algorithm(
            "local_status",
            "",
            r#"<sce:param name="n" type="uint8"/>"#,
            "enum:Status",
            r#"<sce:var name="s" type="enum:Status" init="Status.open"/>
    <sce:assign target="s" expr="n"/>
    <sce:return expr="s"/>"#,
        ),
        "a value that is not of the enum `Status`",
    );
}

/// The algorithm the call cases import: it takes a value of the enum.
fn callee() -> Probe {
    algorithm(
        "probe_callee",
        "",
        r#"<sce:param name="s" type="enum:Status"/>"#,
        "bool",
        r#"<sce:return expr="s === Status.done"/>"#,
    )
}

#[test]
fn a_call_hands_an_enum_only_to_a_parameter_of_it() {
    refused_everywhere(
        &[callee()],
        &algorithm(
            "probe_caller",
            r#"<sce:import kind="algorithm" src="probe_callee.scxml" as="Done"/>"#,
            r#"<sce:param name="n" type="uint8"/>"#,
            "bool",
            r#"<sce:return expr="Done(n)"/>"#,
        ),
        "a value that is not of the enum `Status` handed to `Done`",
    );
}

/// What the rule admits: an enum parameter returned as the enum it is, compared
/// with a variant, and stored in a local of its enum.
#[test]
fn every_backend_accepts_an_enum_where_one_is_declared() {
    accepted_everywhere(
        &[],
        &algorithm(
            "status_kept",
            "",
            r#"<sce:param name="s" type="enum:Status"/>"#,
            "enum:Status",
            r#"<sce:var name="kept" type="enum:Status" init="Status.open"/>
    <sce:if cond="s === Status.done">
      <sce:assign target="kept" expr="s"/>
    </sce:if>
    <sce:return expr="kept"/>"#,
        ),
    );
}
