// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// `datamodel="sce-static"`, State an algorithm takes and gives back
// (docs/SCE_ACCEPTED_SUBSET.md §2.15).
//
// A statechart that keeps a record or a list hands it to an imported algorithm by
// the name of the variable that holds it, and `_event.data` hands the payload of
// the event; what the algorithm returns as a record or a list is taken whole by an
// `<assign>`. These cases pin both halves: every backend lowers the one fixture
// (`scenarios/static_state_algorithm.json` is replayed against each of them by the
// backend's own test), and each way of misusing the call is refused at the call,
// with the reason, and not by the backend that would have generated something that
// does not compile.

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
const SIBLINGS: [&str; 7] = [
    "schema_entry.scxml",
    "schema_total.scxml",
    "algorithm_ledger_add.scxml",
    "algorithm_ledger_total.scxml",
    "algorithm_ledger_amount.scxml",
    "algorithm_ledger_bump.scxml",
    "algorithm_ledger_ids.scxml",
];

/// The fixture machine's text, with `from` replaced by `to` when given.
fn machine(replace: Option<(&str, &str)>) -> String {
    let text = std::fs::read_to_string(fixtures().join("static_state_algorithm.scxml"))
        .expect("the fixture machine");
    match replace {
        Some((from, to)) => {
            assert!(text.contains(from), "the fixture no longer holds `{from}`");
            text.replacen(from, to, 1)
        }
        None => text,
    }
}

/// Run `sce-codegen <args…> <machine>` in a directory holding the machine and the
/// documents it imports, `extra` replacing a sibling of the same name or adding one.
/// Returns `(exit_ok, stdout + stderr)`.
fn run(args: &[&str], machine_text: &str, extra: &[(&str, &str)]) -> (bool, String) {
    run_beside(&SIBLINGS, args, machine_text, extra)
}

/// [`run`], with `siblings` the fixtures copied beside the machine.
fn run_beside(
    siblings: &[&str],
    args: &[&str],
    machine_text: &str,
    extra: &[(&str, &str)],
) -> (bool, String) {
    let dir = tempdir().expect("tempdir");
    for name in siblings {
        std::fs::copy(fixtures().join(name), dir.path().join(name)).expect("copy a sibling");
    }
    for (name, text) in extra {
        std::fs::write(dir.path().join(name), text).expect("write a sibling");
    }
    let path = dir.path().join("probe.scxml");
    std::fs::write(&path, machine_text).expect("write the machine");
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

/// Assert the machine is refused for Rust by the judge, naming `reason`.
fn assert_refused(machine_text: &str, reason: &str) {
    let (ok, out) = run(&["check", "-l", "rust"], machine_text, &[]);
    assert!(!ok, "expected a refusal naming `{reason}`:\n{out}");
    assert!(
        out.contains("\"code\":\"expression/unsupported-construct\""),
        "expected expression/unsupported-construct:\n{out}"
    );
    assert!(out.contains(reason), "expected `{reason}` in:\n{out}");
}

#[test]
fn every_backend_lowers_the_machine_that_hands_algorithms_its_state() {
    let text = machine(None);
    for args in [
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
    ] {
        let (ok, out) = run(&args, &text, &[]);
        assert!(ok, "{args:?} must lower the machine:\n{out}");
    }
}

#[test]
fn the_interpreter_runs_the_machine_that_hands_algorithms_its_state() {
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
    // The call hands the algorithm the variable itself, and the record the event
    // carries as the payload.
    assert!(
        lowered.contains("SceStatic.algorithms.ledger_add(entries,")
            && lowered.contains("SceStatic.algorithms.ledger_amount(_event.data)"),
        "the algorithms are called with the state the machine holds:\n{lowered}"
    );
}

#[test]
fn a_record_of_another_schema_is_refused_where_it_is_handed_over() {
    assert_refused(
        &machine(Some(("Bump(total, 1)", "Bump(last, 1)"))),
        "`last` is a record of `Entry`, and the algorithm takes a record of `Total`",
    );
}

#[test]
fn a_name_that_is_no_variable_is_refused() {
    assert_refused(
        &machine(Some(("TotalOf(entries)", "TotalOf(nothing)"))),
        "`nothing` is not a list variable",
    );
}

#[test]
fn a_value_that_is_computed_is_not_handed_over() {
    assert_refused(
        &machine(Some((
            "TotalOf(entries)",
            "TotalOf(AddEntry(entries, 1, 2))",
        ))),
        "not by an expression that computes one",
    );
}

#[test]
fn a_list_the_variable_cannot_hold_is_refused() {
    assert_refused(
        &machine(Some((
            r#"sce:type="list&lt;record:Entry&gt;" sce:capacity="8""#,
            r#"sce:type="list&lt;record:Entry&gt;" sce:capacity="4""#,
        ))),
        "`AddEntry` may return at most 8, and `entries` holds at most 4",
    );
}

#[test]
fn a_record_or_list_the_algorithm_returns_is_taken_whole_and_not_read_into() {
    assert_refused(
        &machine(Some((
            r#"<assign location="large" expr="large + 1"/>"#,
            r#"<assign location="large" expr="Bump(total, 1).count"/>"#,
        ))),
        "used as a value (it is taken whole by an <assign> to a variable of its type)",
    );
}

#[test]
fn a_payload_is_no_record_on_an_event_that_declares_none() {
    // `bump` carries no schema, so `_event.data` is nothing an algorithm takes.
    assert_refused(
        &machine(Some((
            r#"<assign location="total" expr="Bump(total, 1)"/>"#,
            r#"<assign location="idsum" expr="AmountOf(_event.data)"/>"#,
        ))),
        "the event this transition is on carries no schema",
    );
}

/// A statechart that keeps titled places in a record and a list of records — the
/// records of `sce:std/text/text_slot.scxml`, whose `text` is bounded at 32 UTF-8
/// bytes — and hands them to the standard algorithms over them. `_event.data` is
/// taken into `incoming` first, which is the assignment that holds its text to the
/// bound.
const TEXT_MACHINE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="open" datamodel="sce-static" name="static_state_text">
  <sce:import kind="event-schema" src="sce:std/text/text_slot.scxml" as="Slot"/>
  <sce:import kind="algorithm" src="sce:std/text/text_slot_latest.scxml" as="Latest"/>
  <sce:import kind="algorithm" src="sce:std/text/text_slot_retitled.scxml" as="Retitled"/>
  <sce:import kind="algorithm" src="sce:std/text/text_slot_holds.scxml" as="Holds"/>
  <datamodel>
    <data id="slots" sce:type="list&lt;record:Slot&gt;" sce:capacity="16" sce:direction="out"/>
    <data id="held" sce:type="record:Slot" sce:direction="out">
      <sce:set name="slot" expr="1"/>
      <sce:set name="version" expr="0"/>
      <sce:set name="text" expr="'start'"/>
    </data>
    <data id="incoming" sce:type="record:Slot" sce:direction="out">
      <sce:set name="slot" expr="1"/>
      <sce:set name="version" expr="0"/>
      <sce:set name="text" expr="''"/>
    </data>
    <data id="known" sce:type="uint32" expr="0" sce:direction="out"/>
  </datamodel>
  <state id="open">
    <transition event="text.slot" type="internal">
      <assign location="incoming" expr="_event.data"/>
      <assign location="held" expr="Latest(held, incoming)"/>
      <if cond="Holds(slots, incoming)">
        <assign location="known" expr="known + 1"/>
      </if>
      <sce:append target="slots" expr="incoming"/>
    </transition>
    <transition event="retitle" type="internal">
      <assign location="slots" expr="Retitled(slots)"/>
    </transition>
  </state>
</scxml>
"#;

#[test]
fn a_record_that_holds_text_is_handed_to_an_algorithm_by_every_backend() {
    for args in [
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
    ] {
        let (ok, out) = run(&args, TEXT_MACHINE, &[]);
        assert!(ok, "{args:?} must lower the machine:\n{out}");
    }
}

#[test]
fn the_interpreter_holds_the_text_an_algorithm_writes_to_its_bound() {
    let dir = tempdir().expect("tempdir");
    std::fs::write(dir.path().join("probe.scxml"), TEXT_MACHINE).expect("write the machine");
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
        .collect::<String>()
        // The script engine is handed the attributes as a reader decodes them: the
        // document writes `'` as `&apos;` and `"` as `&quot;`.
        .replace("&apos;", "'")
        .replace("&quot;", "\"");
    // The call hands the algorithm the variable itself, and each text it writes into
    // a record is held to the field's bound where it is written.
    assert!(
        lowered.contains("SceStatic.algorithms.text_slot_latest(held, incoming)"),
        "the algorithm is called with the record the machine holds:\n{lowered}"
    );
    assert!(
        lowered.contains("SceStatic.bounded('untitled', 32)"),
        "the text an algorithm writes is held to the schema's bound:\n{lowered}"
    );
    assert!(
        lowered.contains("SceStatic.bounded(s['text'], 32)"),
        "the text an algorithm copies is held to the bound of the field it lands in:\n{lowered}"
    );
}

#[test]
fn a_payload_that_holds_text_is_not_handed_to_an_algorithm() {
    // The payload's text is held to its schema's bound by the assignment that takes it
    // into a variable, so an algorithm is handed that variable and not the payload.
    let (ok, out) = run(
        &["check", "-l", "rust"],
        &TEXT_MACHINE.replace("Latest(held, incoming)", "Latest(held, _event.data)"),
        &[],
    );
    assert!(!ok, "a payload that holds text must be refused:\n{out}");
    assert!(
        out.contains("the payload of the event holds text"),
        "the refusal names the reason:\n{out}"
    );
}

#[test]
fn a_record_that_holds_bytes_is_not_handed_to_an_algorithm() {
    // The schema the amount algorithm takes a record of now has a byte string: a
    // record is copied into the algorithm's own type and back field by field, and
    // only numbers, truth values, bounded strings and values of an enum copy the same
    // way on every backend. The machine builds its `last` whole, so it gives the new
    // field a value.
    let with_blob = machine(Some((
        "      <sce:set name=\"amount\" expr=\"0\"/>\n    </data>\n    <data id=\"ids\"",
        "      <sce:set name=\"amount\" expr=\"0\"/>\n      <sce:set name=\"blob\" expr=\"'ab'\"/>\n    </data>\n    <data id=\"ids\"",
    )));
    let (ok, out) = run(
        &["check", "-l", "rust"],
        &with_blob,
        &[(
            "schema_entry.scxml",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0"
       sce:kind="event-schema"
       name="schema_entry"
       sce:event-name="ledger.entry">
  <datamodel>
    <data id="id" sce:type="uint32" sce:direction="in"/>
    <data id="amount" sce:type="int32" sce:direction="in"/>
    <data id="blob" sce:type="bytes" sce:max-size="4" sce:direction="in"/>
  </datamodel>
</scxml>
"#,
        )],
    );
    assert!(!ok, "a record with a byte string must be refused:\n{out}");
    assert!(
        out.contains(
            "only records of numbers, truth values, bounded strings and enums are handed to an \
             algorithm"
        ),
        "the refusal names the reason:\n{out}"
    );
}

/// The documents the enum fixture machine imports, beside it: the record whose
/// `status` is a value of the enum, the payload that moves an item, the enum, and
/// the three algorithms over them.
const ENUM_SIBLINGS: [&str; 6] = [
    "schema_item.scxml",
    "schema_item_move.scxml",
    "enum_item_status.scxml",
    "algorithm_item_status_set.scxml",
    "algorithm_item_status_count.scxml",
    "algorithm_item_status_of.scxml",
];

/// The enum fixture machine's text, with each `(from, to)` of `replace` replaced in
/// turn.
fn enum_machine(replace: &[(&str, &str)]) -> String {
    let mut text = std::fs::read_to_string(fixtures().join("static_state_algorithm_enum.scxml"))
        .expect("the enum fixture machine");
    for (from, to) in replace {
        assert!(
            text.contains(from),
            "the enum fixture no longer holds `{from}`"
        );
        text = text.replacen(from, to, 1);
    }
    text
}

/// Assert the enum machine `machine_text`, beside its siblings and `extra`, is
/// refused for Rust by the judge where a call is written, naming `reason`.
fn assert_enum_refused(machine_text: &str, extra: &[(&str, &str)], reason: &str) {
    let (ok, out) = run_beside(
        &ENUM_SIBLINGS,
        &["check", "-l", "rust"],
        machine_text,
        extra,
    );
    assert!(!ok, "expected a refusal naming `{reason}`:\n{out}");
    assert!(
        out.contains("\"code\":\"expression/unsupported-construct\""),
        "expected expression/unsupported-construct:\n{out}"
    );
    assert!(out.contains(reason), "expected `{reason}` in:\n{out}");
}

/// An enum document of `name` with the variants of `enum_item_status.scxml`, closed
/// or, with `open`, declaring `sce:strict-variants="false"`.
fn item_status_like(name: &str, open: bool) -> String {
    let strict = if open {
        "\n       sce:strict-variants=\"false\""
    } else {
        ""
    };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0"
       sce:kind="enum"
       name="{name}"
       sce:underlying-type="uint8"{strict}>
  <datamodel>
    <data id="variants">
      <sce:variant name="open" value="2"/>
      <sce:variant name="in_progress" value="5"/>
      <sce:variant name="done" value="7"/>
    </data>
  </datamodel>
</scxml>
"#
    )
}

#[test]
fn every_backend_lowers_the_machine_that_hands_algorithms_an_enum() {
    let text = enum_machine(&[]);
    for args in [
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
    ] {
        let (ok, out) = run_beside(&ENUM_SIBLINGS, &args, &text, &[]);
        assert!(ok, "{args:?} must lower the machine:\n{out}");
    }
}

#[test]
fn the_interpreter_hands_an_algorithm_an_enum_as_its_declared_name() {
    let dir = tempdir().expect("tempdir");
    for name in ENUM_SIBLINGS {
        std::fs::copy(fixtures().join(name), dir.path().join(name)).expect("copy a sibling");
    }
    std::fs::write(dir.path().join("probe.scxml"), enum_machine(&[])).expect("write the machine");
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
        .collect::<String>()
        .replace("&apos;", "'")
        .replace("&quot;", "\"");
    // The Interpreter holds a value of an enum as the name its variant is declared
    // with, on both sides of the call: the statechart hands the algorithm the name,
    // and the algorithm starts its own value at one.
    assert!(
        lowered.contains("SceStatic.algorithms.item_status_count(items, 'in_progress')"),
        "the variant the statechart names crosses as its declared name:\n{lowered}"
    );
    assert!(
        lowered.contains("var found = 'open';"),
        "a variant the algorithm names is its declared name:\n{lowered}"
    );
}

#[test]
fn a_value_of_another_enum_is_refused_where_it_is_handed_over() {
    let text = enum_machine(&[
        (
            r#"<sce:import kind="enum" src="enum_item_status.scxml" as="Status"/>"#,
            r#"<sce:import kind="enum" src="enum_item_status.scxml" as="Status"/>
  <sce:import kind="enum" src="enum_other.scxml" as="Other"/>"#,
        ),
        (
            "CountStatus(items, Status.open)",
            "CountStatus(items, Other.open)",
        ),
    ]);
    assert_enum_refused(
        &text,
        &[("enum_other.scxml", &item_status_like("enum_other", false))],
        "a value of the enum `Other` handed to `CountStatus`, whose parameter takes a value of \
         the enum `Status`",
    );
}

#[test]
fn a_number_is_no_value_of_the_enum_an_algorithm_takes() {
    assert_enum_refused(
        &enum_machine(&[("CountStatus(items, Status.open)", "CountStatus(items, 2)")]),
        &[],
        "a value that is not of the enum `Status` handed to `CountStatus`",
    );
}

#[test]
fn an_enum_field_of_another_document_is_refused_where_the_schema_is_imported() {
    // The statechart's `Status` is a document with the same variants as the one the
    // schema's field holds, and so another enum: the record is refused where its
    // schema is imported, before any call, as it is for a record no algorithm is
    // handed (`a_record_holds_an_enum_field`).
    let text = enum_machine(&[(
        r#"<sce:import kind="enum" src="enum_item_status.scxml" as="Status"/>"#,
        r#"<sce:import kind="enum" src="enum_item_status_copy.scxml" as="Status"/>"#,
    )]);
    let (ok, out) = run_beside(
        &ENUM_SIBLINGS,
        &["check", "-l", "rust"],
        &text,
        &[(
            "enum_item_status_copy.scxml",
            &item_status_like("enum_item_status_copy", false),
        )],
    );
    assert!(
        !ok,
        "an enum field of another document must be refused:\n{out}"
    );
    assert!(
        out.contains("\"code\":\"scxml/static-datamodel-rule\""),
        "expected scxml/static-datamodel-rule:\n{out}"
    );
    assert!(
        out.contains("which is the schema's own enum document"),
        "the refusal names the rule:\n{out}"
    );
}

#[test]
fn a_value_of_the_enum_an_algorithm_returns_is_no_number() {
    assert_enum_refused(
        &enum_machine(&[(
            r#"<assign location="checks" expr="checks + 1"/>"#,
            r#"<assign location="checks" expr="StatusOf(items, 1)"/>"#,
        )]),
        &[],
        "a value of the enum `Status` where a number or a bool is expected",
    );
}

#[test]
fn an_open_enum_does_not_cross_to_an_algorithm() {
    // A value of an open enum that no variant names has no declared name to cross by.
    // The machine holds no variable of the enum, so the call is what is judged.
    let text = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="board" datamodel="sce-static" name="open_enum_probe">
  <sce:import kind="event-schema" src="schema_item.scxml" as="Item"/>
  <sce:import kind="enum" src="enum_item_status.scxml" as="Status"/>
  <sce:import kind="algorithm" src="algorithm_item_status_count.scxml" as="CountStatus"/>
  <datamodel>
    <data id="items" sce:type="list&lt;record:Item&gt;" sce:capacity="4"/>
    <data id="n" sce:type="uint32" expr="0"/>
  </datamodel>
  <state id="board">
    <transition event="count" type="internal">
      <assign location="n" expr="CountStatus(items, Status.open)"/>
    </transition>
  </state>
</scxml>
"#;
    assert_enum_refused(
        text,
        &[(
            "enum_item_status.scxml",
            &item_status_like("enum_item_status", true),
        )],
        "declares sce:strict-variants=\\\"false\\\": a statechart holds a closed enum",
    );
}
