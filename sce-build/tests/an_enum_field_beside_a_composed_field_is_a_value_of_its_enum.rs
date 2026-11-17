// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// `datamodel="sce-static"` (docs/SCE_ACCEPTED_SUBSET.md §2.15, docs/adr/0086): an
// enum field of a record that also holds a list is a value of its enum, as it is
// in a record whose fields are one entry each.
//
// A record may hold any bounded type, a list of records among them, and a
// schema may grow one in a later revision (`sce:since`). The judge once read the
// enum fields of a record, of the payload and of a loop's record item from the
// view of a schema as one entry per field, which a schema that holds a list has
// none of: `_event.data.<field>` and `<record>.<field>` of such a schema were no
// value of any enum, and a comparison of one with a variant of its own enum was
// refused as "a comparison of an enum value with anything but a value of the
// same enum". The enum fields are now read from the schema whole
// (`enum_field_aliases`, `sce-build/src/forge/static_datamodel.rs`), the one
// place a record variable, the payload and a loop's item ask, for the judge and
// the lowering alike.

use std::path::{Path, PathBuf};
use std::process::Command;

mod common;

use common::composed_lowering::lowering;
use sce_build::forge::event_schema_check::ComposedLowering;
use sce_build::generator::{lowers_static_datamodel_in_part, Language};
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

/// What a command does, the enum the comparisons are of.
const ACTION: &str = r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="enum" name="action" sce:underlying-type="uint8">
  <datamodel>
    <data id="variants">
      <sce:variant name="hold" value="0"/>
      <sce:variant name="apply" value="1"/>
      <sce:variant name="revert" value="2"/>
    </data>
  </datamodel>
</scxml>
"##;

/// Another enum, whose values are no values of `Action`.
const PHASE: &str = r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="enum" name="phase" sce:underlying-type="uint8">
  <datamodel>
    <data id="variants">
      <sce:variant name="early" value="0"/>
      <sce:variant name="late" value="1"/>
    </data>
  </datamodel>
</scxml>
"##;

/// One step of a command: a record of a number and a number that may be absent.
const STEP: &str = r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="schema_step" sce:event-name="step.taken">
  <datamodel>
    <data id="at" sce:type="uint32" sce:direction="in"/>
    <data id="shift" sce:type="optional&lt;int64&gt;" sce:direction="in"/>
  </datamodel>
</scxml>
"##;

/// A command: an enum field of the first revision, and a list of steps the second
/// revision added, under a bound a build may move.
const COMMAND: &str = r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="schema_command"
       sce:event-name="command.sent" sce:revision="2">
  <sce:import src="action.scxml" kind="enum" as="Action"/>
  <sce:import src="schema_step.scxml" kind="event-schema" as="Step"/>
  <datamodel>
    <data id="action" sce:type="enum:Action" sce:direction="in"/>
    <data id="steps" sce:type="list&lt;record:Step&gt;[step_limit=64]" sce:direction="in" sce:since="2"/>
  </datamodel>
</scxml>
"##;

/// The files every statechart below imports, beside it.
const SIBLINGS: [(&str, &str); 4] = [
    ("action.scxml", ACTION),
    ("phase.scxml", PHASE),
    ("schema_step.scxml", STEP),
    ("schema_command.scxml", COMMAND),
];

/// A statechart that imports the command, its step and `Action` (and `Phase` when
/// `phase`), declares `variables` beside a counter `applied` and an `Action`
/// variable `taken`, and whose one state holds `transitions`.
fn statechart(phase: bool, variables: &str, transitions: &str) -> String {
    let phase = if phase {
        r#"<sce:import kind="enum" src="phase.scxml" as="Phase"/>"#
    } else {
        ""
    };
    format!(
        r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static">
  <sce:import kind="event-schema" src="schema_command.scxml" as="Command"/>
  <sce:import kind="event-schema" src="schema_step.scxml" as="Step"/>
  <sce:import kind="enum" src="action.scxml" as="Action"/>
  {phase}
  <datamodel>
    <data id="applied" sce:type="uint32" expr="0"/>
    <data id="taken" sce:type="enum:Action" expr="Action.hold"/>
    {variables}
  </datamodel>
  <state id="s">
    {transitions}
  </state>
</scxml>
"##
    )
}

/// A statechart whose `command.sent` transition is guarded by `cond`, beside one
/// that keeps the command's action when it is not `hold` and counts its steps.
fn payload_statechart(phase: bool, cond: &str) -> String {
    statechart(
        phase,
        r#"<data id="steps" sce:type="uint32" expr="0"/>"#,
        &format!(
            r#"<transition event="command.sent" cond="{cond}" type="internal">
      <assign location="applied" expr="applied + 1"/>
    </transition>
    <transition event="command.sent" type="internal">
      <if cond="_event.data.action !== Action.hold">
        <assign location="taken" expr="_event.data.action"/>
      </if>
      <assign location="steps" expr="len(_event.data.steps)"/>
    </transition>"#
        ),
    )
}

/// A statechart that holds a command in the variable `held` declares, and counts
/// a `check` event whose guard is `cond`.
fn record_statechart(phase: bool, held: &str, cond: &str) -> String {
    statechart(
        phase,
        held,
        &format!(
            r#"<transition event="check" cond="{cond}" type="internal">
      <assign location="applied" expr="applied + 1"/>
    </transition>"#
        ),
    )
}

/// `held`, an `optional<record:Command>` that starts absent and a lift would write
/// whole.
const OPTIONAL_HELD: &str =
    r#"<data id="held" sce:type="optional&lt;record:Command&gt;" expr="null"/>"#;

/// `held`, a `record:Command` built where it is declared: its action given, its
/// list of steps starting empty.
const BUILT_HELD: &str = r#"<data id="held" sce:type="record:Command">
      <sce:set name="action" expr="Action.hold"/>
    </data>"#;

/// Write the siblings and `doc` (as `probe.scxml`) into a new directory, which
/// is kept as long as the handle is.
fn beside(doc: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempdir().expect("tempdir");
    for (name, text) in SIBLINGS {
        std::fs::write(dir.path().join(name), text).expect("write a sibling");
    }
    let path = dir.path().join("probe.scxml");
    std::fs::write(&path, doc).expect("write the probe");
    (dir, path)
}

/// `sce-codegen <args…> probe.scxml` with JSON diagnostics, the siblings beside
/// it: `(exit_ok, stderr + stdout)`.
fn run(args: &[&str], doc: &str) -> (bool, String) {
    let (_dir, path) = beside(doc);
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

/// `sce-codegen generate -l <language>` of `doc` into a directory of its own:
/// `(exit_ok, stderr + stdout)`.
fn generate(language: &str, doc: &str) -> (bool, String) {
    let out_dir = tempdir().expect("tempdir");
    let out_path = out_dir.path().to_str().expect("a UTF-8 path");
    let mut args = vec!["generate", "-l", language, "-o", out_path];
    if language == "go" {
        args.extend(["--go-module-prefix", "example.test/generated"]);
    }
    run(&args, doc)
}

/// What the judge says of an enum value compared with anything but a value of its
/// own enum (the refusal of `sce-build/src/forge/static_enum.rs`).
const NOT_THE_SAME_ENUM: &str =
    "a comparison of an enum value with anything but a value of the same enum";

/// Every backend that generates a `sce-static` statechart, whole or in part, whose
/// row of the one table of what is lowered (`ComposedLowering::of`) takes
/// `position`, by the name the command line takes.
fn generated_lanes(position: fn(ComposedLowering) -> bool) -> Vec<&'static str> {
    let mut lanes = lowering(position);
    lanes.extend(
        Language::ALL
            .iter()
            .copied()
            .filter(|language| lowers_static_datamodel_in_part(*language))
            .filter(|language| position(ComposedLowering::of(*language)))
            .map(Language::canonical_name),
    );
    lanes
}

/// Pinned by `enum_field_aliases` and `Judge::open_payload`
/// (`static_datamodel.rs`): the payload of an event whose schema grew a list of
/// records in its second revision is read with its enum field a value of its
/// enum, compared with `===` and `!==` to a variant and stored in a variable of
/// it. Every generated backend that reads a composed payload lowers the guard
/// (a `sce-static` statechart has no script engine to fall back on, so a
/// guard a backend could not lower is a refusal of the document), and the
/// Interpreter's lowering writes it into the transition's `cond`.
#[test]
fn an_enum_field_of_a_payload_that_grew_a_list_is_compared_on_every_lane() {
    let doc = payload_statechart(false, "_event.data.action === Action.apply");
    let (ok, out) = run(&["check"], &doc);
    assert!(
        ok,
        "the judge takes an enum field beside a list of records:\n{out}"
    );
    let lanes = generated_lanes(|row| row.reads);
    assert!(
        lanes.len() >= 6,
        "every backend that lowers sce-static reads a composed payload: {lanes:?}"
    );
    for language in lanes {
        let (ok, out) = generate(language, &doc);
        assert!(
            ok,
            "{language}: the guard on an enum field beside a list is lowered:\n{out}"
        );
        assert!(
            !out.contains(NOT_THE_SAME_ENUM),
            "{language}: the enum field is a value of its enum:\n{out}"
        );
        assert!(
            out.contains("\"needs_script_engine\":false"),
            "{language}: the comparison needs no script engine:\n{out}"
        );
    }
    let (_dir, path) = beside(&doc);
    let lowered =
        sce_build::forge::static_js::lower_file(path.to_str().expect("a UTF-8 path"), Vec::new())
            .unwrap_or_else(|e| panic!("the Interpreter lowers the guard: {e:?}"));
    let document = roxmltree::Document::parse(&lowered).expect("a well-formed lowering");
    let guards: Vec<&str> = document
        .descendants()
        .filter(|n| {
            n.is_element()
                && n.tag_name().name() == "transition"
                && n.attribute("event") == Some("command.sent")
        })
        .filter_map(|n| n.attribute("cond"))
        .collect();
    assert!(
        guards.iter().any(|cond| cond.contains("apply")),
        "the Interpreter's transition is guarded by the comparison with the variant: \
         {guards:?}\n{lowered}"
    );
}

/// Pinned by `Judge::open_payload` (`static_datamodel.rs`): the field is a value of
/// its own enum and of no other, so a variant of another enum, or a number, beside
/// it is refused as before.
#[test]
fn an_enum_field_of_a_payload_that_grew_a_list_is_no_value_of_another_enum() {
    for cond in [
        "_event.data.action === Phase.early",
        "_event.data.action !== Phase.late",
        "_event.data.action === 1",
    ] {
        let (ok, out) = run(&["check"], &payload_statechart(true, cond));
        assert!(!ok, "`{cond}` is refused:\n{out}");
        assert!(
            out.contains(NOT_THE_SAME_ENUM),
            "`{cond}` is refused as a comparison with another enum's value:\n{out}"
        );
    }
}

/// Pinned by `enum_variables` (`static_datamodel.rs`): an `optional<record:R>` whose
/// `R` holds a list reads its enum field under its present value
/// (`held.value.action`), a value of its enum, where a test proves it present; and
/// each backend whose row holds such a variable lowers the guard.
#[test]
fn an_enum_field_of_an_optional_record_that_holds_a_list_is_compared() {
    let doc = record_statechart(
        false,
        OPTIONAL_HELD,
        "held !== null &amp;&amp; held.value.action === Action.revert",
    );
    let (ok, out) = run(&["check"], &doc);
    assert!(
        ok,
        "the judge takes an enum field of a record that holds a list:\n{out}"
    );
    let holding = generated_lanes(|row| row.records);
    assert!(
        !holding.is_empty(),
        "some backend holds a variable of a record that holds a list"
    );
    for language in holding {
        let (ok, out) = generate(language, &doc);
        assert!(
            ok,
            "{language}: the guard on an enum field of the record is lowered:\n{out}"
        );
    }
}

/// Pinned by `enum_variables` (`static_datamodel.rs`): the record's enum field is a
/// value of its own enum alone.
#[test]
fn an_enum_field_of_an_optional_record_that_holds_a_list_is_no_value_of_another_enum() {
    let doc = record_statechart(
        true,
        OPTIONAL_HELD,
        "held !== null &amp;&amp; held.value.action === Phase.early",
    );
    let (ok, out) = run(&["check"], &doc);
    assert!(!ok, "a variant of another enum is refused:\n{out}");
    assert!(
        out.contains(NOT_THE_SAME_ENUM),
        "the refusal is the comparison with another enum's value:\n{out}"
    );
}

/// Pinned by `enum_variables` (`static_datamodel.rs`): a `record:R` built where it is
/// declared, whose `R` holds a list, reads its enum field as a value of its enum.
/// Whether a backend holds such a variable is the composed gate's to say
/// (`ComposedLowering`), after the judge; the comparison is never what refuses it.
#[test]
fn an_enum_field_of_a_built_record_that_holds_a_list_is_not_what_refuses_it() {
    let doc = record_statechart(false, BUILT_HELD, "held.action === Action.apply");
    let (_, out) = run(&["check"], &doc);
    assert!(
        !out.contains(NOT_THE_SAME_ENUM),
        "the record's enum field is a value of its enum:\n{out}"
    );
}

/// Pinned by the loop's enum paths in `Judge::foreach` (`static_datamodel.rs`): the
/// item of a `<foreach>` over a payload's list of records, each of which holds a
/// list of its own (`event_batch_item.scxml`), reads its enum field as a value of
/// its enum.
#[test]
fn an_enum_field_of_a_loop_item_that_holds_a_list_is_compared() {
    let fixtures = repo_root().join("sce-build/tests/fixtures/static_datamodel");
    let dir = tempdir().expect("tempdir");
    for name in [
        "event_batch.scxml",
        "event_batch_item.scxml",
        "enum_signal_level.scxml",
    ] {
        std::fs::copy(fixtures.join(name), dir.path().join(name)).expect("copy a fixture");
    }
    let doc = |level: &str| {
        format!(
            r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static">
  <sce:import kind="event-schema" src="event_batch.scxml" as="Batch"/>
  <sce:import kind="event-schema" src="event_batch_item.scxml" as="Item"/>
  <sce:import kind="enum" src="enum_signal_level.scxml" as="SignalLevel"/>
  <datamodel>
    <data id="highs" sce:type="uint32" expr="0"/>
  </datamodel>
  <state id="s">
    <transition event="batch.sent" type="internal">
      <foreach array="_event.data.items" item="row">
        <if cond="row.level === {level}">
          <assign location="highs" expr="highs + 1"/>
        </if>
      </foreach>
    </transition>
  </state>
</scxml>
"##
        )
    };
    let check = |text: &str| {
        let path = dir.path().join("probe.scxml");
        std::fs::write(&path, text).expect("write the probe");
        let out = Command::new(sce_codegen_bin())
            .arg("--workspace-root")
            .arg(repo_root())
            .arg("--error-format=json")
            .arg("check")
            .arg(&path)
            .output()
            .expect("invoke sce-codegen");
        (
            out.status.success(),
            String::from_utf8_lossy(&out.stderr).into_owned()
                + &String::from_utf8_lossy(&out.stdout),
        )
    };
    let (ok, out) = check(&doc("SignalLevel.high"));
    assert!(
        ok,
        "a loop item's enum field beside a list is a value of its enum:\n{out}"
    );
    let (ok, out) = check(&doc("2"));
    assert!(!ok, "a number is no value of the item's enum:\n{out}");
    assert!(
        out.contains(NOT_THE_SAME_ENUM),
        "the refusal is the comparison with a number:\n{out}"
    );
}
