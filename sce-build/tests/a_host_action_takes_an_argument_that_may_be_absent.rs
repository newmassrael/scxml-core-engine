// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A host action's argument that may be absent (docs/adr/0055, step 10): a
//! `<sce:arg>` of a `sce-static` statechart whose expression is of `optional<T>` is
//! the parameter of the host method in the backend's own optional type over `T`
//! (`Option<T>`, `T?`, `optional.Of[T]`, `Optional[T]`, `std::optional<T>`,
//! `sce_optional_<elem>_t`), and the Interpreter's host is handed `null` for an absent
//! one. The signature tells `int32` from `optional<int32>`, so two calls of one action
//! name that disagree are the signature conflict any two types make, and nothing is
//! coerced; `null` as an argument has no type and a value of an enum or a record is no
//! host argument, present or absent. C11 declares its struct per element, a text or a
//! byte string over a buffer of a bound that a host method's parameter does not
//! declare, so an optional text or byte string is refused there by name.
//!
//! The scenario `static_host_call.json` replays the machine on the seven engines; this
//! file holds what the build answers before anything runs.
//!
//! # What was wrong
//!
//! Measured in the code on 2026-10-28: an argument of an optional type was refused
//! for every backend with "a host method takes no argument that may be absent yet",
//! the Interpreter's executor raised `error.execution` for an argument that was
//! `null`, and a call whose argument read `_event.data.due.value` under the test that
//! proves it present was judged fit by validation and then panicked where the call was
//! rendered, since the rendering judged it without the facts the `cond` proved.

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
/// schema whose payload has an optional day, and the enum that schema and a probe
/// of an optional value of an enum import.
const SIBLINGS: [&str; 2] = ["schema_due.scxml", "enum_item_status.scxml"];

/// `sce-codegen <args…> <file>` in a directory holding the siblings and `documents`.
/// Returns `(exit_ok, stderr + stdout, the text of every file written)`.
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
    if matches!(args.first(), Some(&"generate") | Some(&"lower")) {
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

/// The `generate` arguments of one backend by its canonical name.
fn generate(language: &'static str) -> Vec<&'static str> {
    let mut args = vec!["generate", "-l", language];
    if language == "go" {
        args.extend(["--go-module-prefix", "example.test/generated"]);
    }
    args
}

/// The enum and the schema of a payload with an optional day, which a probe that
/// reads that payload or holds a value of the enum imports.
const IMPORTS: &str = r#"<sce:import kind="enum" src="enum_item_status.scxml" as="Status"/>
  <sce:import kind="event-schema" src="schema_due.scxml" as="Due"/>"#;

/// A `sce-static` statechart that declares `data` beside a day and a flag that may be
/// absent and a day that never is, and whose one state, `idle`, has `transitions`.
fn statechart(data: &str, transitions: &str) -> String {
    importing("", data, transitions)
}

/// [`statechart`] with the `imports` written before its data.
fn importing(imports: &str, data: &str, transitions: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="host_optional_probe">
  {imports}
  <datamodel>
    <data id="due" sce:type="optional&lt;int32&gt;" expr="null"/>
    <data id="urgent" sce:type="optional&lt;bool&gt;" expr="null"/>
    <data id="plain" sce:type="int32" expr="0"/>
    {data}
  </datamodel>
  <state id="idle">
    {transitions}
  </state>
</scxml>
"#
    )
}

/// A `<transition>` on `event` that calls the host operation `action` with each of
/// `args`, written `(name, expression)`.
fn calling(event: &str, action: &str, args: &[(&str, &str)]) -> String {
    let args: String = args
        .iter()
        .map(|(name, expr)| format!(r#"<sce:arg name="{name}" expr="{expr}"/>"#))
        .collect();
    format!(
        r#"<transition event="{event}"><sce:action name="{action}" effect="notice">{args}</sce:action></transition>"#
    )
}

/// The probe of numbers and `bool`s: `show(day, urgent)` is handed two variables that
/// may be absent.
fn numbers() -> String {
    statechart(
        "",
        &calling("go", "show", &[("day", "due"), ("urgent", "urgent")]),
    )
}

/// What each backend's host interface declares for [`numbers`]: the method, in the
/// language's own convention and its own optional type over the type of the present
/// value.
const NUMBER_METHODS: [(&str, &str); 6] = [
    (
        "rust",
        "fn show(&mut self, day: Option<i32>, urgent: Option<bool>);",
    ),
    ("kotlin", "fun show(day: Int?, urgent: Boolean?)"),
    (
        "go",
        "Show(day optional.Of[int32], urgent optional.Of[bool])",
    ),
    (
        "python",
        "def show(self, day: Optional[int], urgent: Optional[bool]) -> None:",
    ),
    (
        "cpp",
        "virtual void show(std::optional<int32_t> day, std::optional<bool> urgent) = 0;",
    ),
    (
        "c11",
        "void (*show)(void *user_data, sce_optional_int32_t day, sce_optional_bool_t urgent);",
    ),
];

/// Every backend declares the host method of an argument that may be absent in its
/// own optional type, the one a statechart's variable of the same type is held in, so
/// the variable is handed to the host as it is.
#[test]
fn every_backend_declares_an_optional_parameter_in_its_own_optional_type() {
    let probe = numbers();
    for (language, method) in NUMBER_METHODS {
        let (ok, out, written) = run(
            &generate(language),
            &[("probe.scxml", probe.as_str())],
            "probe.scxml",
        );
        assert!(
            ok,
            "{language} lowers a host action's optional argument:\n{out}"
        );
        assert!(
            written.contains(method),
            "the {language} host interface declares `{method}`:\n{written}"
        );
    }
}

/// An argument that is not optional keeps the type it had, so a host that takes a day
/// and one that takes a day that may be absent are two methods of two names.
#[test]
fn an_argument_that_is_never_absent_keeps_its_plain_parameter() {
    let probe = statechart(
        "",
        &format!(
            "{}{}",
            calling("go", "show", &[("day", "due")]),
            calling("plain", "show_plain", &[("day", "plain")])
        ),
    );
    for (language, optional, plain) in [
        (
            "rust",
            "fn show(&mut self, day: Option<i32>);",
            "fn show_plain(&mut self, day: i32);",
        ),
        ("kotlin", "fun show(day: Int?)", "fun showPlain(day: Int)"),
        ("go", "Show(day optional.Of[int32])", "ShowPlain(day int32)"),
        (
            "python",
            "def show(self, day: Optional[int]) -> None:",
            "def show_plain(self, day: int) -> None:",
        ),
        (
            "cpp",
            "virtual void show(std::optional<int32_t> day) = 0;",
            "virtual void showPlain(int32_t day) = 0;",
        ),
        (
            "c11",
            "void (*show)(void *user_data, sce_optional_int32_t day);",
            "void (*show_plain)(void *user_data, int32_t day);",
        ),
    ] {
        let (ok, out, written) = run(
            &generate(language),
            &[("probe.scxml", probe.as_str())],
            "probe.scxml",
        );
        assert!(ok, "{language} lowers both calls:\n{out}");
        for needle in [optional, plain] {
            assert!(
                written.contains(needle),
                "the {language} host interface declares `{needle}`:\n{written}"
            );
        }
    }
}

/// A text and a byte string that may be absent are the backend's optional type over
/// the type a present one is passed as: a Rust host borrows the present text and bytes
/// (`Option<&str>`, `Option<&[u8]>`), as an algorithm's parameter borrows them.
#[test]
fn an_optional_text_and_byte_string_are_the_backends_optional_over_what_a_present_one_is() {
    let probe = statechart(
        r#"<data id="note" sce:type="optional&lt;string&gt;" sce:capacity="8" expr="null"/>
    <data id="blob" sce:type="optional&lt;bytes&gt;" sce:capacity="8" expr="null"/>"#,
        &format!(
            "{}{}",
            calling("say", "say", &[("note", "note")]),
            calling("blob", "send_blob", &[("blob", "blob")])
        ),
    );
    for (language, text, bytes) in [
        (
            "rust",
            "fn say(&mut self, note: Option<&str>);",
            "fn send_blob(&mut self, blob: Option<&[u8]>);",
        ),
        (
            "kotlin",
            "fun say(note: String?)",
            "fun sendBlob(blob: ByteArray?)",
        ),
        (
            "go",
            "Say(note optional.Of[string])",
            "SendBlob(blob optional.Of[[]byte])",
        ),
        (
            "python",
            "def say(self, note: Optional[str]) -> None:",
            "def send_blob(self, blob: Optional[bytes]) -> None:",
        ),
        (
            "cpp",
            "virtual void say(std::optional<std::string> note) = 0;",
            "virtual void sendBlob(std::optional<std::vector<uint8_t>> blob) = 0;",
        ),
    ] {
        let (ok, out, written) = run(
            &generate(language),
            &[("probe.scxml", probe.as_str())],
            "probe.scxml",
        );
        assert!(
            ok,
            "{language} lowers an optional text and byte string:\n{out}"
        );
        for needle in [text, bytes] {
            assert!(
                written.contains(needle),
                "the {language} host interface declares `{needle}`:\n{written}"
            );
        }
    }
}

/// C11 declares one struct per element, and a text or a byte string is a buffer of a
/// bound its value declares (`sce:capacity`, `sce:max-size`): a host method's
/// parameter declares none, so the argument is refused by name, in the wording every
/// refusal of an optional value takes.
#[test]
fn c11_refuses_an_optional_text_and_byte_string_by_name() {
    for (data, action, argument, element) in [
        (
            r#"<data id="note" sce:type="optional&lt;string&gt;" sce:capacity="8" expr="null"/>"#,
            "say",
            "note",
            "optional<string>",
        ),
        (
            r#"<data id="blob" sce:type="optional&lt;bytes&gt;" sce:capacity="8" expr="null"/>"#,
            "send_blob",
            "blob",
            "optional<bytes>",
        ),
    ] {
        let probe = statechart(data, &calling("go", action, &[(argument, argument)]));
        let (ok, out, _) = run(
            &generate("c11"),
            &[("probe.scxml", probe.as_str())],
            "probe.scxml",
        );
        assert!(!ok, "C11 refuses {element} as a host argument:\n{out}");
        let operation = format!("host operation `{action}`");
        for needle in [
            "generate/unsupported-feature",
            operation.as_str(),
            element,
            "an optional value has no C11 lowering yet",
        ] {
            assert!(out.contains(needle), "expected `{needle}` in:\n{out}");
        }
    }
}

/// A field of the payload that may be absent is an argument as it is, and `.value` of
/// it is one of `T` where the `cond` of the transition proves it present: the call is
/// rendered under the facts validation judged it by, where it used to be judged
/// without them.
#[test]
fn a_payload_field_that_may_be_absent_is_an_argument_and_its_proven_value_is_a_plain_one() {
    let probe = importing(
        IMPORTS,
        "",
        &format!(
            r#"{}<transition event="due.set" cond="_event.data.due !== null"><sce:action name="show_present" effect="notice"><sce:arg name="day" expr="_event.data.due.value"/></sce:action></transition>"#,
            calling("due.set", "show", &[("day", "_event.data.due")])
        ),
    );
    for (language, optional, present) in [
        (
            "rust",
            "fn show(&mut self, day: Option<i64>);",
            "fn show_present(&mut self, day: i64);",
        ),
        (
            "kotlin",
            "fun show(day: Long?)",
            "fun showPresent(day: Long)",
        ),
        (
            "go",
            "Show(day optional.Of[int64])",
            "ShowPresent(day int64)",
        ),
        (
            "python",
            "def show(self, day: Optional[int]) -> None:",
            "def show_present(self, day: int) -> None:",
        ),
        (
            "cpp",
            "virtual void show(std::optional<int64_t> day) = 0;",
            "virtual void showPresent(int64_t day) = 0;",
        ),
        (
            "c11",
            "void (*show)(void *user_data, sce_optional_int64_t day);",
            "void (*show_present)(void *user_data, int64_t day);",
        ),
    ] {
        let (ok, out, written) = run(
            &generate(language),
            &[("probe.scxml", probe.as_str())],
            "probe.scxml",
        );
        assert!(ok, "{language} renders both calls:\n{out}");
        for needle in [optional, present] {
            assert!(
                written.contains(needle),
                "the {language} host interface declares `{needle}`:\n{written}"
            );
        }
    }
}

/// The Interpreter's lowering leaves each argument for its engine to compute, and the
/// host is handed `null` for one that is absent, so a document with an optional
/// argument lowers as one with any other.
#[test]
fn the_interpreter_lowers_a_host_action_with_an_optional_argument() {
    let probe = numbers();
    let (ok, out, written) = run(
        &["lower"],
        &[("probe.scxml", probe.as_str())],
        "probe.scxml",
    );
    assert!(ok, "the Interpreter lowers the document:\n{out}");
    assert!(
        written.contains(r#"<sce:action name="show""#),
        "the lowered document keeps the action:\n{written}"
    );
}

/// `sce-codegen check -l rust` of `text`: refused, with `code` and each of `needles`.
fn refused(text: &str, code: &str, needles: &[&str]) {
    let (ok, out, _) = run(
        &["check", "-l", "rust"],
        &[("probe.scxml", text)],
        "probe.scxml",
    );
    assert!(!ok, "expected a refusal with {code}:\n{out}");
    assert!(out.contains(code), "expected the code {code} in:\n{out}");
    for needle in needles {
        assert!(out.contains(needle), "expected `{needle}` in:\n{out}");
    }
}

/// One host method serves every call site of a name, so an argument that is
/// `optional<int32>` at one and `int32` at another is two signatures, and the second
/// call is refused naming both: nothing is coerced to make them agree.
#[test]
fn two_calls_that_disagree_on_whether_an_argument_may_be_absent_are_a_conflict() {
    let probe = statechart(
        "",
        &format!(
            "{}{}",
            calling("go", "show", &[("day", "due")]),
            calling("plain", "show", &[("day", "plain")])
        ),
    );
    refused(
        &probe,
        "validation/native-action-signature-conflict",
        &["(int32) here disagree with (optional<int32>)"],
    );
}

/// `null` is the absent value of the optional place it stands in, and an argument
/// stands in none, so it has no type a host method could declare.
#[test]
fn null_is_no_argument() {
    let probe = statechart("", &calling("go", "show", &[("day", "null")]));
    refused(
        &probe,
        "validation/native-action-argument",
        &["has no type a host method could declare"],
    );
}

/// A value of an enum is no host argument, present or absent, so an optional one is
/// refused as a value of one is.
#[test]
fn an_optional_value_of_an_enum_is_no_host_argument() {
    let probe = importing(
        r#"<sce:import kind="enum" src="enum_item_status.scxml" as="Status"/>"#,
        r#"<data id="status" sce:type="optional&lt;enum:Status&gt;" expr="null"/>"#,
        &calling("go", "show", &[("status", "status")]),
    );
    refused(
        &probe,
        "validation/native-action-argument",
        &["takes no value of an enum or of a record"],
    );
}
