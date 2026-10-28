// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The Python backend's step 9 of docs/adr/0055: a record that may be absent,
//! `optional<record:X>`, in a `sce-static` statechart and in an algorithm's slots,
//! and every optional element an algorithm holds (a text, a byte string, a value of an
//! enum, a record). Python holds an optional value in `Optional[T]`, `None` absent and
//! the value itself present, over the type its present value has there: a record is
//! the statechart's frozen dataclass in a machine and the schema's payload class in an
//! algorithm.
//!
//! The scenario `static_optional_record.json` replays the generated machine
//! (`backends/python/tests/integration/static_datamodel/test_static_scenarios.py`);
//! this file holds what `sce-codegen generate -l python` writes for it, which is read
//! before anything runs.

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

/// `sce-codegen generate -l python <file>` in a directory holding the fixture
/// documents named in `siblings` and the `documents` written as given. Returns
/// `(exit_ok, stderr + stdout, the text of every file written)`.
fn generate(siblings: &[&str], documents: &[(&str, &str)], file: &str) -> (bool, String, String) {
    let dir = tempdir().expect("tempdir");
    for name in siblings {
        std::fs::copy(fixtures().join(name), dir.path().join(name)).expect("copy a sibling");
    }
    for (name, text) in documents {
        std::fs::write(dir.path().join(name), text).expect("write a document");
    }
    // The document asked for is a fixture unless the test wrote it.
    if !documents.iter().any(|(name, _)| *name == file) {
        std::fs::copy(fixtures().join(file), dir.path().join(file)).expect("copy the document");
    }
    let out_dir = tempdir().expect("tempdir");
    let out = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(["generate", "-l", "python"])
        .arg(dir.path().join(file))
        .arg("-o")
        .arg(out_dir.path())
        .output()
        .expect("invoke sce-codegen");
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

/// `written` holds each of `needles`, or the test fails naming it.
fn holds_every(written: &str, needles: &[&str]) {
    for needle in needles {
        assert!(
            written.contains(needle),
            "the generated Python holds `{needle}`:\n{written}"
        );
    }
}

// ── The machine of the scenario ──────────────────────────────────────────

/// The machine of `static_optional_record.json` is generated, and spells an optional
/// record as the scenario reads it: the variable in `Optional[<record>]` starting at
/// `None`, a record taken from a list's element by the runtime's checked read, a field
/// of the present record read as the record's member and written by replacing the
/// frozen record, an algorithm's answer built into the machine's own record where it
/// is present, and the saved form of the variable as `null` or the record.
#[test]
fn the_optional_record_scenario_machine_is_generated_in_optional_of_its_record() {
    let (ok, out, written) = generate(
        &["schema_item_pick.scxml", "algorithm_item_pick_first.scxml"],
        &[],
        "static_optional_record.scxml",
    );
    assert!(ok, "Python generates the machine:\n{out}");
    holds_every(
        &written,
        &[
            // The variable: the one optional type over the machine's record, absent.
            "v_pick: Optional[StaticOptionalRecordItemRecord] = None",
            "v_found: Optional[StaticOptionalRecordItemRecord] = None",
            // Presence is an identity test, and a field of a present record is the
            // member of the record the variable is.
            "self.v_pick is not None",
            "self.v_pick.count",
            // A field is written by replacing the frozen record with one whose
            // field is changed.
            "self.v_pick = sce_dataclasses.replace(self.v_pick, count=(",
            "self.v_pick = sce_dataclasses.replace(self.v_pick, title=(",
            // An element of a list, whole, by the checked read.
            "sce_algorithm.at(self.v_items, 1)",
            // What the algorithm answers: absent as it is, and a record the machine
            // builds from the algorithm's where it is present.
            "def _call_FirstAtLeast__v(p0, p1):",
            "return (None if r is None else StaticOptionalRecordItemRecord(id=r.id, \
             title=r.title, count=r.count))",
            // The saved form: `null` or the record's own.
            "sce_saved.saved_write_optional(policy.v_pick, lambda v: v.to_saved())",
            "StaticOptionalRecordItemRecord.from_saved)",
        ],
    );
}

// ── An algorithm holds one ───────────────────────────────────────────────

/// The algorithm of the scenario answers an optional record: its return and its local
/// are `Optional[<payload>]`, the local starts at `None`, the test is an identity
/// test, and a record put into the local is a copy of its own, since the payload class
/// is mutable and the item the loop reads is another value.
#[test]
fn an_algorithm_that_answers_an_optional_record_is_generated_in_optional_of_the_payload() {
    let (ok, out, written) = generate(
        &["schema_item_pick.scxml"],
        &[],
        "algorithm_item_pick_first.scxml",
    );
    assert!(ok, "Python generates the algorithm:\n{out}");
    holds_every(
        &written,
        &[
            "from sce_forge_runtime import algorithm as sce_algorithm",
            "from typing import Optional",
            "found: Optional[",
            ") -> Optional[",
            "found is None and ",
            "it.count >= least",
            "found = sce_algorithm.copied(it)",
            "return found",
        ],
    );
}

/// An algorithm that takes an optional record and keeps one in a local reads and writes
/// the fields of the present record as the members of the payload object: a number
/// by plain assignment and a text by the write that holds it to the bound its field
/// declares.
const PICK_COUNT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="pick_count" version="1.0">
  <sce:import kind="event-schema" src="schema_item_pick.scxml" as="Item"/>
  <sce:signature>
    <sce:param name="x" type="optional&lt;record:Item&gt;"/>
    <sce:param name="items" type="list&lt;record:Item&gt;"/>
    <sce:return type="int32"/>
  </sce:signature>
  <sce:body>
    <sce:var name="held" type="optional&lt;record:Item&gt;" init="null"/>
    <sce:foreach item="it" in="items">
      <sce:if cond="held === null">
        <sce:assign target="held" expr="it"/>
      </sce:if>
    </sce:foreach>
    <sce:if cond="held !== null">
      <sce:assign target="held.value.count" expr="7"/>
      <sce:assign target="held.value.title" expr="'bumped'"/>
      <sce:return expr="held.value.count"/>
    </sce:if>
    <sce:if cond="x !== null">
      <sce:return expr="x.value.count"/>
    </sce:if>
    <sce:return expr="-1"/>
  </sce:body>
</scxml>
"#;

#[test]
fn an_algorithm_reads_and_writes_the_fields_of_the_optional_record_it_holds() {
    let (ok, out, written) = generate(
        &["schema_item_pick.scxml"],
        &[("algorithm_pick_count.scxml", PICK_COUNT)],
        "algorithm_pick_count.scxml",
    );
    assert!(ok, "Python generates the algorithm:\n{out}");
    holds_every(
        &written,
        &[
            // The parameter is the optional type over the payload class.
            "x: Optional[",
            "held: Optional[",
            "held = sce_algorithm.copied(it)",
            // A field of the present record is the member of the object.
            "held.count = 7",
            "held.title = 'bumped'",
            "return held.count",
            "return x.count",
        ],
    );
}

// ── An optional text, byte string, enum and record in an algorithm's slots ──

/// An algorithm whose parameter, return and local are each `optional<T>` is generated
/// with the one optional type over the type `T` is held in: `str`, `bytes`, the enum
/// document's class and the schema's payload class.
#[test]
fn every_optional_element_an_algorithm_holds_is_spelled_in_optional_of_its_type() {
    for (written, present, imports) in [
        ("optional&lt;string&gt;", "Optional[str]", ""),
        ("optional&lt;bytes&gt;", "Optional[bytes]", ""),
        (
            "optional&lt;enum:Status&gt;",
            "Optional[",
            r#"  <sce:import kind="enum" src="enum_item_status.scxml" as="Status"/>"#,
        ),
        (
            "optional&lt;record:Item&gt;",
            "Optional[",
            r#"  <sce:import kind="event-schema" src="schema_item_pick.scxml" as="Item"/>"#,
        ),
    ] {
        let text = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="slot_probe" version="1.0">
{imports}
  <sce:signature>
    <sce:param name="given" type="{written}"/>
    <sce:return type="{written}"/>
  </sce:signature>
  <sce:body>
    <sce:var name="held" type="{written}" init="null"/>
    <sce:if cond="given !== null">
      <sce:return expr="given"/>
    </sce:if>
    <sce:return expr="held"/>
  </sce:body>
</scxml>
"#
        );
        let (ok, out, generated) = generate(
            &["enum_item_status.scxml", "schema_item_pick.scxml"],
            &[("slot_probe.scxml", &text)],
            "slot_probe.scxml",
        );
        assert!(
            ok,
            "Python spells `{written}` on an algorithm's slots:\n{out}"
        );
        holds_every(
            &generated,
            &[
                &format!("given: {present}"),
                &format!("held: {present}"),
                "from typing import Optional",
            ],
        );
        assert!(
            generated.contains(") -> Optional["),
            "the return is the optional type too:\n{generated}"
        );
    }
}

/// The unit of an event schema whose fields may be absent holds each in the one
/// optional type over its present value's: a number, a text and a value of an enum.
#[test]
fn the_unit_of_a_schema_with_an_optional_text_and_enum_field_is_generated() {
    let (ok, out, written) = generate(&["enum_item_status.scxml"], &[], "schema_due.scxml");
    assert!(ok, "Python generates the schema:\n{out}");
    holds_every(
        &written,
        &[
            "due: Optional[int]",
            "title: Optional[str]",
            "status: Optional[",
            "from typing import Final, Optional",
        ],
    );
}

/// An algorithm whose record has an optional text and an optional enum field reads each
/// as an optional value, and a statechart that hands it a list of such records crosses
/// the enum field by the declared name of its variant where it is present and leaves
/// it absent where it is not: the variant is mapped and `None` is not.
#[test]
fn a_call_crosses_an_optional_enum_field_of_a_record_by_its_declared_name() {
    let algorithm = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="due_probe" version="1.0">
  <sce:import kind="event-schema" src="schema_due.scxml" as="Due"/>
  <sce:signature>
    <sce:param name="dues" type="list&lt;record:Due&gt;"/>
    <sce:return type="int32"/>
  </sce:signature>
  <sce:body>
    <sce:var name="found" type="int32" init="0"/>
    <sce:foreach item="d" in="dues">
      <sce:if cond="d.title !== null">
        <sce:assign target="found" expr="1"/>
      </sce:if>
    </sce:foreach>
    <sce:return expr="found"/>
  </sce:body>
</scxml>
"#;
    let machine = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="due_probe_machine">
  <sce:import kind="enum" src="enum_item_status.scxml" as="Status"/>
  <sce:import kind="event-schema" src="schema_due.scxml" as="Due"/>
  <sce:import kind="algorithm" src="algorithm_due_probe.scxml" as="DueCount"/>
  <datamodel>
    <data id="dues" sce:type="list&lt;record:Due&gt;" sce:capacity="3"/>
    <data id="last" sce:type="enum:Status" expr="Status.open"/>
    <data id="seen" sce:type="int32" expr="0"/>
  </datamodel>
  <state id="idle">
    <transition event="go">
      <assign location="seen" expr="DueCount(dues)"/>
    </transition>
  </state>
</scxml>
"#;
    let (ok, out, written) = generate(
        &["enum_item_status.scxml", "schema_due.scxml"],
        &[
            ("algorithm_due_probe.scxml", algorithm),
            ("due_probe_machine.scxml", machine),
        ],
        "algorithm_due_probe.scxml",
    );
    assert!(ok, "Python generates the algorithm:\n{out}");
    holds_every(&written, &["if d.title is not None:"]);
    let (ok, out, written) = generate(
        &["enum_item_status.scxml", "schema_due.scxml"],
        &[
            ("algorithm_due_probe.scxml", algorithm),
            ("due_probe_machine.scxml", machine),
        ],
        "due_probe_machine.scxml",
    );
    assert!(ok, "Python generates the machine:\n{out}");
    holds_every(
        &written,
        &[
            "(lambda sce_present: None if sce_present is None else {",
            "}[sce_present])(e.status)",
            "title=e.title",
        ],
    );
}

// ── A statechart calls an algorithm that takes an optional record ────────

/// A statechart hands an optional record slot the variable that holds one, a record
/// variable and `null`, and the adapter of each call takes the argument as it is held:
/// `None` where the argument is absent, and the record built as an algorithm's
/// record is where it is present.
#[test]
fn a_call_hands_an_optional_record_slot_none_or_a_record_built_for_the_algorithm() {
    let machine = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="pick_probe_machine">
  <sce:import kind="event-schema" src="schema_item_pick.scxml" as="Item"/>
  <sce:import kind="algorithm" src="algorithm_pick_count.scxml" as="PickCount"/>
  <datamodel>
    <data id="pick" sce:type="optional&lt;record:Item&gt;" expr="null"/>
    <data id="draft" sce:type="record:Item">
      <sce:set name="id" expr="0"/>
      <sce:set name="title" expr="'draft'"/>
      <sce:set name="count" expr="0"/>
    </data>
    <data id="items" sce:type="list&lt;record:Item&gt;" sce:capacity="3"/>
    <data id="seen" sce:type="int32" expr="0"/>
  </datamodel>
  <state id="idle">
    <transition event="go">
      <assign location="seen" expr="PickCount(pick, items)"/>
      <assign location="seen" expr="PickCount(null, items)"/>
      <assign location="seen" expr="PickCount(draft, items)"/>
    </transition>
  </state>
</scxml>
"#;
    let (ok, out, written) = generate(
        &["schema_item_pick.scxml"],
        &[
            ("algorithm_pick_count.scxml", PICK_COUNT),
            ("pick_probe_machine.scxml", machine),
        ],
        "pick_probe_machine.scxml",
    );
    assert!(ok, "Python generates the machine:\n{out}");
    holds_every(
        &written,
        &[
            // The optional variable and the record variable are one adapter, which
            // tests the argument; `null` is the absent value itself.
            "def _call_PickCount__vv(p0, p1):",
            "def _call_PickCount__nv(p0, p1):",
            "(None if p0 is None else ",
            "_call_PickCount__nv(None, self.v_items)",
            "_call_PickCount__vv(self.v_pick, self.v_items)",
            "_call_PickCount__vv(self.v_draft, self.v_items)",
        ],
    );
}
