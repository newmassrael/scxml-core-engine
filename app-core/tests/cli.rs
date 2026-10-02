// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! `sce-work`, as a process a caller that is not Rust runs.
//!
//! The authoring MCP writes the works folder through this binary, so what it
//! promises is what is held here: a JSON line out, a refusal as JSON on stderr, a
//! status that tells a conflict from any other refusal, and two processes that
//! save from one base never both succeeding.

mod common;

use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

use serde_json::{json, Value};

fn run(root: &Path, command: &str, args: Option<&Value>) -> Output {
    let mut process = Command::new(env!("CARGO_BIN_EXE_sce-work"));
    process
        .arg("--root")
        .arg(root)
        .args(["call", command, "--args-stdin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = process.spawn().expect("run sce-work");
    {
        let mut stdin = child.stdin.take().expect("stdin");
        if let Some(args) = args {
            stdin
                .write_all(args.to_string().as_bytes())
                .expect("write the arguments");
        }
    }
    child.wait_with_output().expect("wait for sce-work")
}

fn ok(output: &Output) -> Value {
    assert!(
        output.status.success(),
        "status {:?}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("one JSON value on stdout")
}

fn refusal(output: &Output) -> (i32, Value) {
    assert!(output.stdout.is_empty(), "a refusal wrote to stdout");
    let body: Value = serde_json::from_slice(&output.stderr).expect("a JSON refusal on stderr");
    assert_eq!(body["v"], 1);
    (
        output.status.code().expect("a status"),
        body["error"].clone(),
    )
}

fn create(root: &Path, title: &str) -> String {
    let work = ok(&run(root, "create_work", Some(&json!({ "title": title }))));
    work["id"].as_str().expect("an id").to_string()
}

#[test]
fn describe_lists_the_commands_and_the_root() {
    let root = common::scratch("cli-describe");
    let described = ok(&run(&root, "describe", None));
    assert_eq!(described["command_set_version"], 1);
    let names: Vec<&str> = described["commands"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n.as_str().unwrap())
        .collect();
    assert_eq!(names, sce_app_core::COMMANDS);
    assert_eq!(described["root"], root.display().to_string());
}

#[test]
fn a_work_is_created_saved_and_read_through_the_command() {
    let root = common::scratch("cli-flow");
    let id = create(&root, "Door controller");

    let saved = ok(&run(
        &root,
        "save_source",
        Some(&json!({ "id": id, "text": "The door opens when a card is shown." })),
    ));
    assert_eq!(saved["outcome"], "saved");
    assert_eq!(saved["parent"], Value::Null);
    let revision = saved["revision"].as_str().unwrap().to_string();

    let read = ok(&run(&root, "read_source", Some(&json!({ "id": id }))));
    assert_eq!(
        read["source"]["text"],
        "The door opens when a card is shown."
    );
    assert_eq!(read["source"]["revision"], revision);

    let work = ok(&run(&root, "read_work", Some(&json!({ "id": id }))));
    assert_eq!(work["head"], revision);
    let history = ok(&run(&root, "history", Some(&json!({ "id": id }))));
    assert_eq!(history["entries"].as_array().unwrap().len(), 1);
    let listed = ok(&run(&root, "list_works", None));
    assert_eq!(listed["works"].as_array().unwrap().len(), 1);
}

#[test]
fn a_text_with_a_newline_and_a_script_other_than_latin_survives_the_pipe() {
    let root = common::scratch("cli-text");
    // Hangul syllables, three UTF-8 bytes to one scalar each, written as escapes
    // so the source stays ASCII; what is measured is the width, not the script.
    let id = create(&root, "\u{BB38} \u{C81C}\u{C5B4}");
    let text =
        "\u{CCAB} \u{C904}\r\n\u{B450} \u{BC88}\u{C9F8} \u{C904}\n\"\u{C778}\u{C6A9}\" \\ \u{B05D}";
    assert!(
        text.chars().any(|c| c.len_utf8() == 3),
        "the fixture must hold a multi-byte scalar, or this test passes on ASCII"
    );
    ok(&run(
        &root,
        "save_source",
        Some(&json!({ "id": id, "text": text })),
    ));
    let read = ok(&run(&root, "read_source", Some(&json!({ "id": id }))));
    assert_eq!(read["source"]["text"], text);
}

#[test]
fn a_stale_base_exits_with_three_and_says_what_is_current() {
    let root = common::scratch("cli-conflict");
    let id = create(&root, "Door");
    let first = ok(&run(
        &root,
        "save_source",
        Some(&json!({ "id": id, "text": "one" })),
    ));
    let one = first["revision"].clone();
    let second = ok(&run(
        &root,
        "save_source",
        Some(&json!({ "id": id, "text": "two", "base": one })),
    ));

    let output = run(
        &root,
        "save_source",
        Some(&json!({ "id": id, "text": "three", "base": one })),
    );
    let (status, error) = refusal(&output);
    assert_eq!(status, 3);
    assert_eq!(error["kind"], "conflict");
    assert_eq!(error["detail"]["base"], one);
    assert_eq!(error["detail"]["current"], second["revision"]);
}

#[test]
fn other_refusals_exit_with_one_and_name_their_kind() {
    let root = common::scratch("cli-refusals");
    for (command, args, kind) in [
        ("read_work", json!({ "id": "nothing-here" }), "not-found"),
        ("read_work", json!({ "id": "../etc" }), "invalid-id"),
        ("create_work", json!({ "title": "" }), "invalid-title"),
        (
            "create_work",
            json!({ "title": "x", "surplus": 1 }),
            "bad-request",
        ),
        ("no_such_command", json!({}), "unknown-command"),
    ] {
        let (status, error) = refusal(&run(&root, command, Some(&args)));
        assert_eq!(status, 1, "{command} {args}");
        assert_eq!(error["kind"], kind, "{command} {args}");
    }
}

#[test]
fn arguments_that_are_not_json_are_a_plain_refusal() {
    let root = common::scratch("cli-not-json");
    let mut child = Command::new(env!("CARGO_BIN_EXE_sce-work"))
        .arg("--root")
        .arg(&root)
        .args(["call", "list_works", "--args-stdin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"{ nope").unwrap();
    let (status, error) = refusal(&child.wait_with_output().unwrap());
    assert_eq!(status, 1);
    assert_eq!(error["kind"], "bad-request");
}

#[test]
fn of_several_processes_saving_from_one_base_exactly_one_succeeds() {
    const PROCESSES: usize = 6;
    let root = common::scratch("cli-race");
    let id = create(&root, "Door");
    let base = ok(&run(
        &root,
        "save_source",
        Some(&json!({ "id": id, "text": "base" })),
    ))["revision"]
        .clone();

    let children: Vec<_> = (0..PROCESSES)
        .map(|n| {
            let root = root.clone();
            let args = json!({ "id": id, "text": format!("process {n}"), "base": base });
            std::thread::spawn(move || run(&root, "save_source", Some(&args)))
        })
        .collect();
    let outputs: Vec<Output> = children.into_iter().map(|h| h.join().unwrap()).collect();

    let codes: Vec<i32> = outputs.iter().map(|o| o.status.code().unwrap()).collect();
    assert_eq!(codes.iter().filter(|c| **c == 0).count(), 1, "{codes:?}");
    assert!(codes.iter().all(|c| *c == 0 || *c == 3), "{codes:?}");
    let history = ok(&run(&root, "history", Some(&json!({ "id": id }))));
    assert_eq!(history["entries"].as_array().unwrap().len(), 2);
}
