// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! An algorithm's body calls another algorithm whose slot is `optional<enum:X>` or
//! `optional<record:X>` (docs/adr/0055, steps 8 and 9): the argument is held to the
//! enum document or the schema the slot names, as it is for a slot of the enum or
//! of the record, and a call that fits is lowered on every backend.
//!
//! # What was wrong
//!
//! Measured in the code on 2026-10-28: a slot of an optional value of an enum was
//! never named to the importing body's enum rule, so a value of the enum it takes
//! was refused as "an enum value used as an operand or an argument" and one of
//! another enum was refused by no rule that named either; and a slot of an optional
//! record gave the callee no signature at all, so an algorithm that took one could
//! be called by a host and by no other algorithm.

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

/// The documents of the fixture directory the probes import, copied beside them: two
/// enums and two record schemas.
const SIBLINGS: [&str; 4] = [
    "enum_item_status.scxml",
    "enum_signal_level.scxml",
    "schema_goal.scxml",
    "schema_day.scxml",
];

/// The `generate` or `check` arguments of each generated backend, by the language it
/// names: the six backends an algorithm is written for.
fn every_backend(command: &'static str) -> Vec<(&'static str, Vec<&'static str>)> {
    let plain = |lang: &'static str| (lang, vec![command, "-l", lang]);
    vec![
        plain("rust"),
        plain("kotlin"),
        plain("python"),
        plain("c11"),
        plain("cpp"),
        (
            "go",
            vec![
                command,
                "-l",
                "go",
                "--go-module-prefix",
                "example.test/generated",
            ],
        ),
    ]
}

/// An algorithm document named `name` (written to `<name>.scxml`) whose imports are
/// `imports`, whose one parameter `given` is `param`, which returns `ret` and whose
/// body is `body`.
fn algorithm(name: &str, imports: &str, param: &str, ret: &str, body: &str) -> (String, String) {
    (
        format!("{name}.scxml"),
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="{name}" version="1.0">
{imports}
  <sce:signature>
    <sce:param name="given" type="{param}"/>
    <sce:return type="{ret}"/>
  </sce:signature>
  <sce:body>
    {body}
  </sce:body>
</scxml>
"#
        ),
    )
}

/// The callee of the enum cases: it takes and answers an optional value of the enum
/// `enum_item_status.scxml`.
fn status_next() -> (String, String) {
    algorithm(
        "status_next",
        r#"  <sce:import kind="enum" src="enum_item_status.scxml" as="Status"/>"#,
        "optional&lt;enum:Status&gt;",
        "optional&lt;enum:Status&gt;",
        r#"<sce:return expr="given"/>"#,
    )
}

/// The callee of the record cases: it takes and answers an optional record of the
/// schema `schema_goal.scxml`.
fn goal_next() -> (String, String) {
    algorithm(
        "goal_next",
        r#"  <sce:import kind="event-schema" src="schema_goal.scxml" as="Goal"/>"#,
        "optional&lt;record:Goal&gt;",
        "optional&lt;record:Goal&gt;",
        r#"<sce:return expr="given"/>"#,
    )
}

/// `sce-codegen <args…> <file>` in a directory that holds the siblings, `beside` and
/// `probe`: `(exit_ok, stderr + stdout)`. A command that writes takes a fresh `-o`.
fn run(args: &[&str], beside: &[(String, String)], probe: &(String, String)) -> (bool, String) {
    let dir = tempdir().expect("tempdir");
    for name in SIBLINGS {
        std::fs::copy(fixtures().join(name), dir.path().join(name)).expect("copy a sibling");
    }
    for (name, text) in beside.iter().chain(std::iter::once(probe)) {
        std::fs::write(dir.path().join(name), text).expect("write a document");
    }
    let out_dir = tempdir().expect("tempdir");
    let mut command = Command::new(sce_codegen_bin());
    command
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(args)
        .arg(dir.path().join(&probe.0));
    if args.first() == Some(&"generate") {
        command.arg("-o").arg(out_dir.path());
    }
    let out = command.output().expect("invoke sce-codegen");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout),
    )
}

/// `probe`, with `beside` written next to it, is refused on every backend as
/// `expression/unsupported-construct`, and each of `fragments` is in what it says: the
/// verdict is the build's and does not depend on the language asked for.
fn refused_everywhere(beside: &[(String, String)], probe: &(String, String), fragments: &[&str]) {
    let mut wrong = Vec::new();
    for (lang, args) in every_backend("check") {
        let (ok, out) = run(&args, beside, probe);
        if ok {
            wrong.push(format!("[{lang}] accepted:\n{out}"));
        } else if !out.contains("\"code\":\"expression/unsupported-construct\"") {
            wrong.push(format!("[{lang}] refused by another code:\n{out}"));
        } else if let Some(missing) = fragments.iter().find(|piece| !out.contains(**piece)) {
            wrong.push(format!(
                "[{lang}] refused without saying `{missing}`:\n{out}"
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// `probe`, with `beside` written next to it, is generated by every backend and
/// lowered for the Interpreter.
fn lowered_everywhere(beside: &[(String, String)], probe: &(String, String)) {
    let mut refused = Vec::new();
    for (lang, args) in every_backend("generate") {
        let (ok, out) = run(&args, beside, probe);
        if !ok {
            refused.push(format!("[{lang}]:\n{out}"));
        }
    }
    let (ok, out) = run(&["lower-algorithm"], beside, probe);
    if !ok {
        refused.push(format!("[interpreter]:\n{out}"));
    }
    assert!(refused.is_empty(), "{}", refused.join("\n"));
}

// ── An optional value of an enum ─────────────────────────────────────────

/// A value of an enum of another document is no argument of a slot of an optional
/// value of `enum_item_status.scxml`, and the refusal names the enum the caller holds
/// and the document the slot is of.
#[test]
fn an_enum_of_another_document_is_refused_where_an_optional_enum_is_declared() {
    refused_everywhere(
        &[status_next()],
        &algorithm(
            "level_caller",
            r#"  <sce:import kind="enum" src="enum_signal_level.scxml" as="Level"/>
  <sce:import kind="algorithm" src="status_next.scxml" as="Next"/>"#,
            "optional&lt;enum:Level&gt;",
            "optional&lt;enum:Level&gt;",
            r#"<sce:return expr="Next(Level.low)"/>"#,
        ),
        &[
            "a value of the enum `Level` handed to `Next`",
            "whose parameter takes an optional value of the enum",
            "enum_item_status.scxml",
        ],
    );
}

/// An optional value of an enum of another document is no argument of it either: the
/// slot is of the document, whatever the value is held in.
#[test]
fn an_optional_enum_of_another_document_is_refused_where_an_optional_enum_is_declared() {
    refused_everywhere(
        &[status_next()],
        &algorithm(
            "level_relay",
            r#"  <sce:import kind="enum" src="enum_signal_level.scxml" as="Level"/>
  <sce:import kind="algorithm" src="status_next.scxml" as="Next"/>"#,
            "optional&lt;enum:Level&gt;",
            "optional&lt;enum:Level&gt;",
            r#"<sce:return expr="Next(given)"/>"#,
        ),
        &[
            "an optional value of the enum `Level` handed to `Next`",
            "enum_item_status.scxml",
        ],
    );
}

/// A number is none of an optional value of an enum, `null` apart.
#[test]
fn a_number_is_refused_where_an_optional_enum_is_declared() {
    refused_everywhere(
        &[status_next()],
        &algorithm(
            "number_caller",
            r#"  <sce:import kind="enum" src="enum_item_status.scxml" as="Item"/>
  <sce:import kind="algorithm" src="status_next.scxml" as="Next"/>"#,
            "optional&lt;enum:Item&gt;",
            "optional&lt;enum:Item&gt;",
            r#"<sce:return expr="Next(3)"/>"#,
        ),
        &["a value that is neither `null` nor of the enum `Item`"],
    );
}

/// An optional value of the enum the slot is of, held under the alias the caller
/// imports its document by, is handed on and answered back: every backend generates
/// the call and the Interpreter lowers it. The caller names the enum `Item`, the callee
/// `Status`; the document is one.
#[test]
fn every_backend_lowers_a_call_that_hands_on_an_optional_enum_of_the_slots_document() {
    lowered_everywhere(
        &[status_next()],
        &algorithm(
            "item_relay",
            r#"  <sce:import kind="enum" src="enum_item_status.scxml" as="Item"/>
  <sce:import kind="algorithm" src="status_next.scxml" as="Next"/>"#,
            "optional&lt;enum:Item&gt;",
            "optional&lt;enum:Item&gt;",
            r#"<sce:return expr="Next(given)"/>"#,
        ),
    );
}

// ── An optional record ───────────────────────────────────────────────────

/// An optional record of another schema is no argument of a slot of an optional
/// record of `schema_goal.scxml`, and the refusal names both schemas.
#[test]
fn a_record_of_another_schema_is_refused_where_an_optional_record_is_declared() {
    refused_everywhere(
        &[goal_next()],
        &algorithm(
            "day_caller",
            r#"  <sce:import kind="event-schema" src="schema_goal.scxml" as="Goal"/>
  <sce:import kind="event-schema" src="schema_day.scxml" as="Day"/>
  <sce:import kind="algorithm" src="goal_next.scxml" as="Next"/>"#,
            "optional&lt;record:Day&gt;",
            "optional&lt;record:Goal&gt;",
            r#"<sce:return expr="Next(given)"/>"#,
        ),
        &[
            "`given`, an optional record of event-schema `schema_day`",
            "handed to `Next`",
            "takes an optional record of event-schema `schema_goal`",
        ],
    );
}

/// What a call answers is an optional record of the schema its callee names: put into
/// a place that holds one of another schema it is refused, naming both.
#[test]
fn a_call_answering_an_optional_record_lands_only_in_a_place_of_its_schema() {
    refused_everywhere(
        &[goal_next()],
        &algorithm(
            "day_answer",
            r#"  <sce:import kind="event-schema" src="schema_goal.scxml" as="Goal"/>
  <sce:import kind="event-schema" src="schema_day.scxml" as="Day"/>
  <sce:import kind="algorithm" src="goal_next.scxml" as="Next"/>"#,
            "optional&lt;record:Goal&gt;",
            "optional&lt;record:Day&gt;",
            r#"<sce:return expr="Next(given)"/>"#,
        ),
        &[
            "the call of `Next`, which answers an optional record of event-schema `schema_goal`",
            "a place that holds an optional record of event-schema `schema_day`",
        ],
    );
}

/// An optional record of the schema the slot is of is handed on and answered back: the
/// callee has a signature another algorithm calls it by, and every backend generates
/// the call and the Interpreter lowers it.
#[test]
fn every_backend_lowers_a_call_that_hands_on_an_optional_record_of_the_slots_schema() {
    lowered_everywhere(
        &[goal_next()],
        &algorithm(
            "goal_relay",
            r#"  <sce:import kind="event-schema" src="schema_goal.scxml" as="Target"/>
  <sce:import kind="algorithm" src="goal_next.scxml" as="Next"/>"#,
            "optional&lt;record:Target&gt;",
            "optional&lt;record:Target&gt;",
            r#"<sce:return expr="Next(given)"/>"#,
        ),
    );
}
