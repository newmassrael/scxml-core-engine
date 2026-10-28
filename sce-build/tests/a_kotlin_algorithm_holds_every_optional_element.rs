// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The Kotlin backend holds every element of `optional<T>` in an algorithm
//! (docs/adr/0055, step 9): a parameter, a return and a local of a text, a byte
//! string, a value of an enum or a record are `T?` over the type their present value
//! is; a record an algorithm holds may have a field that is an optional text or a
//! value of an enum, which it writes as a statechart's record does, a text only from
//! a literal or a text field the build bounds and from `null`; and a statechart
//! hands such a record over and takes an optional text back through an adapter that
//! carries each as it is, a value of an enum by the declared name of its variant.
//!
//! This file holds the text the generator writes for each, and the refusals that
//! remain by name: the elements a statechart's call does not cross
//! (`StaticValueSignature`), which are named where the call is read.
//!
//! # What was wrong
//!
//! Measured in the code on 2026-10-28: the Kotlin backend spelled a number and a
//! `bool` in an algorithm's slot and refused every other element by name, and an
//! algorithm whose record had an optional text or enum field was refused as a whole,
//! though a statechart's variable of the same type was already `T?`.

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
/// record `schema_due` (an optional day, text and value of an enum), the enum it
/// holds and the record `schema_goal`.
const SIBLINGS: [&str; 3] = [
    "schema_due.scxml",
    "enum_item_status.scxml",
    "schema_goal.scxml",
];

/// `sce-codegen <command> -l kotlin <file>` in a directory holding the siblings and
/// `documents`, `file` being one of them. Returns `(exit_ok, stderr + stdout, the text
/// of every file written)`.
fn run_kotlin(command: &str, documents: &[(&str, &str)], file: &str) -> (bool, String, String) {
    let dir = tempdir().expect("tempdir");
    for name in SIBLINGS {
        std::fs::copy(fixtures().join(name), dir.path().join(name)).expect("copy a sibling");
    }
    for (name, text) in documents {
        std::fs::write(dir.path().join(name), text).expect("write a document");
    }
    let out_dir = tempdir().expect("tempdir");
    let mut run = Command::new(sce_codegen_bin());
    run.arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args([command, "-l", "kotlin"])
        .arg(dir.path().join(file));
    if command == "generate" {
        run.arg("-o").arg(out_dir.path());
    }
    let out = run.output().expect("invoke sce-codegen");
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

/// `written` holds each of `needles`, or the test names the one it lacks and shows
/// what was written.
fn holds_each(written: &str, needles: &[&str]) {
    for needle in needles {
        assert!(
            written.contains(needle),
            "the generated Kotlin holds `{needle}`:\n{written}"
        );
    }
}

/// An algorithm of no body to speak of whose one parameter, return and local are
/// declared `param`, `ret` and `local`, returning the local. `imports` are the
/// documents its slots name.
fn slots(imports: &str, param: &str, ret: &str, local: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="slot_probe" version="1.0">
{imports}  <sce:signature>
    <sce:param name="given" type="{param}"/>
    <sce:return type="{ret}"/>
  </sce:signature>
  <sce:body>
    <sce:var name="held" type="{local}" init="null"/>
    <sce:return expr="held"/>
  </sce:body>
</scxml>
"#
    )
}

// ── The slots ────────────────────────────────────────────────────────────

/// A parameter, the return and a local of `optional<T>` over a text, a byte string,
/// a value of an enum and a record are `T?` over the type of the present value, and
/// the local starts absent.
#[test]
fn kotlin_spells_every_optional_element_in_an_algorithms_slots() {
    let status = "  <sce:import kind=\"enum\" src=\"enum_item_status.scxml\" as=\"Status\"/>\n";
    let goal = "  <sce:import kind=\"event-schema\" src=\"schema_goal.scxml\" as=\"Goal\"/>\n";
    for (imports, written, spelled) in [
        ("", "optional&lt;string&gt;", "String?"),
        ("", "optional&lt;bytes&gt;", "ByteArray?"),
        (status, "optional&lt;enum:Status&gt;", "EnumItemStatus?"),
        (goal, "optional&lt;record:Goal&gt;", "SchemaGoalPayload?"),
    ] {
        let (ok, out, written_kotlin) = run_kotlin(
            "generate",
            &[(
                "slot_probe.scxml",
                &slots(imports, written, written, written),
            )],
            "slot_probe.scxml",
        );
        assert!(ok, "Kotlin generates `{written}` on every slot:\n{out}");
        let parameter = format!("given: {spelled}");
        let local = format!("var held: {spelled} = null");
        let ret = format!("): {spelled} {{");
        holds_each(
            &written_kotlin,
            &[parameter.as_str(), local.as_str(), ret.as_str()],
        );
    }
}

/// A test of an optional text, an optional byte string and an optional value of an
/// enum is `!= null`, a proven read is the one function every proven read is spelled
/// with, `??` is the elvis, and a value of the enum is compared with its own variant:
/// the judge registers the enum an optional parameter holds at its present value.
#[test]
fn kotlin_reads_an_optional_element_where_it_is_proven_and_with_a_default() {
    let body = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="read_probe" version="1.0">
  <sce:import kind="enum" src="enum_item_status.scxml" as="Status"/>
  <sce:signature>
    <sce:param name="status" type="optional&lt;enum:Status&gt;"/>
    <sce:param name="label" type="optional&lt;string&gt;"/>
    <sce:param name="frame" type="optional&lt;bytes&gt;"/>
    <sce:return type="int32"/>
  </sce:signature>
  <sce:body>
    <sce:if cond="status !== null &amp;&amp; status.value === Status.done">
      <sce:return expr="1"/>
    </sce:if>
    <sce:if cond="label !== null &amp;&amp; len(label.value) &gt; 3">
      <sce:return expr="2"/>
    </sce:if>
    <sce:if cond="(label ?? 'none') === 'none'">
      <sce:return expr="3"/>
    </sce:if>
    <sce:if cond="frame !== null &amp;&amp; len(frame.value) &gt; 0">
      <sce:return expr="4"/>
    </sce:if>
    <sce:return expr="0"/>
  </sce:body>
</scxml>
"#;
    let (ok, out, written) = run_kotlin(
        "generate",
        &[("read_probe.scxml", body)],
        "read_probe.scxml",
    );
    assert!(ok, "Kotlin generates the algorithm:\n{out}");
    holds_each(
        &written,
        &[
            "status: EnumItemStatus?",
            "label: String?",
            "frame: ByteArray?",
            "status != null",
            "com.sce.forge.runtime.SceOptional.proven(status) == EnumItemStatus.DONE",
            "com.sce.forge.runtime.SceOptional.proven(label)",
            "label ?: \"none\"",
            "frame != null",
            "com.sce.forge.runtime.SceOptional.proven(frame)",
        ],
    );
}

// ── A record with an optional text and an optional value of an enum ───────

/// An algorithm that takes a record of `schema_due` (an optional day, text and value
/// of an enum), and runs `body`, answering an optional text.
fn on_due(body: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="due_label" version="1.0">
  <sce:import kind="event-schema" src="schema_due.scxml" as="Due"/>
  <sce:import kind="enum" src="enum_item_status.scxml" as="Status"/>
  <sce:signature>
    <sce:param name="item" type="record:Due"/>
    <sce:return type="optional&lt;string&gt;"/>
  </sce:signature>
  <sce:body>
    {body}
  </sce:body>
</scxml>
"#
    )
}

/// The record of `schema_due` carries its optional text and value of an enum in the
/// data class its unit generates, and an algorithm that holds the record reads each
/// as the optional value it is.
#[test]
fn kotlin_generates_the_record_of_an_optional_text_and_an_optional_enum() {
    let (ok, out, written) = run_kotlin("generate", &[], "schema_due.scxml");
    assert!(ok, "Kotlin generates the schema:\n{out}");
    holds_each(
        &written,
        &[
            "val due: Long?",
            "val title: String?",
            "val status: EnumItemStatus?",
        ],
    );
    let (ok, out, written) = run_kotlin(
        "generate",
        &[(
            "algorithm_due_label.scxml",
            &on_due(
                r#"<sce:if cond="item.status !== null &amp;&amp; item.status.value === Status.done">
      <sce:return expr="item.title"/>
    </sce:if>
    <sce:return expr="null"/>"#,
            ),
        )],
        "algorithm_due_label.scxml",
    );
    assert!(ok, "Kotlin generates the algorithm:\n{out}");
    holds_each(
        &written,
        &[
            "item: SchemaDuePayload",
            "): String? {",
            "item.status != null",
            "com.sce.forge.runtime.SceOptional.proven(item.status) == EnumItemStatus.DONE",
            "return item.title",
        ],
    );
}

/// A record local of `schema_due`, `mine`, built whole: an algorithm's parameter is
/// read-only, so a record the body writes is one of its own.
const MINE: &str = r#"<sce:var name="mine" type="record:Due">
      <sce:set name="id" expr="0"/>
      <sce:set name="due" expr="null"/>
      <sce:set name="title" expr="'first'"/>
      <sce:set name="status" expr="null"/>
    </sce:var>"#;

/// A record is built whole with an optional text from a literal the build bounds and
/// with `null` where a field may be absent, and a field of an optional text is written
/// from a literal and from `null`, and a field of an optional value of an enum from a
/// variant: each is the record replaced by itself with that field changed.
#[test]
fn kotlin_writes_an_optional_text_field_from_a_bounded_text_or_null() {
    let body = format!(
        r#"{MINE}
    <sce:assign target="mine.title" expr="'abc'"/>
    <sce:assign target="mine.status" expr="Status.done"/>
    <sce:if cond="mine.title !== null">
      <sce:assign target="mine.title" expr="null"/>
    </sce:if>
    <sce:return expr="mine.title"/>"#
    );
    let (ok, out, written) = run_kotlin(
        "generate",
        &[("algorithm_due_label.scxml", &on_due(&body))],
        "algorithm_due_label.scxml",
    );
    assert!(ok, "Kotlin generates the algorithm:\n{out}");
    holds_each(
        &written,
        &[
            // Built whole: a text from its literal, the rest absent.
            "var mine = SchemaDuePayload(",
            "due = null",
            "title = \"first\"",
            "status = null",
            // Written a field at a time.
            "mine = mine.copy(title = \"abc\")",
            "mine = mine.copy(status = EnumItemStatus.DONE)",
            "mine = mine.copy(title = null)",
            "return mine.title",
        ],
    );
}

/// A literal past the bound of an optional text field is refused where it is
/// written, as one past a text field's is: no text of an algorithm's record is held
/// past what its schema declares.
#[test]
fn kotlin_refuses_a_text_past_the_bound_of_an_optional_text_field() {
    let body = format!(
        r#"{MINE}
    <sce:assign target="mine.title" expr="'a text that is longer than sixteen bytes'"/>
    <sce:return expr="null"/>"#
    );
    let (ok, out, _) = run_kotlin(
        "generate",
        &[("algorithm_due_label.scxml", &on_due(&body))],
        "algorithm_due_label.scxml",
    );
    assert!(!ok, "a text past the bound is refused:\n{out}");
    assert!(
        out.contains("a string of at most 16 UTF-8 bytes") && out.contains("`title`"),
        "refused naming the bound of the field:\n{out}"
    );
    // ... and so is one past it where the record is built whole.
    let body = r#"<sce:var name="mine" type="record:Due">
      <sce:set name="id" expr="0"/>
      <sce:set name="due" expr="null"/>
      <sce:set name="title" expr="'a text that is longer than sixteen bytes'"/>
      <sce:set name="status" expr="null"/>
    </sce:var>
    <sce:return expr="null"/>"#;
    let (ok, out, _) = run_kotlin(
        "generate",
        &[("algorithm_due_label.scxml", &on_due(body))],
        "algorithm_due_label.scxml",
    );
    assert!(!ok, "a text past the bound is refused:\n{out}");
    assert!(
        out.contains("a string of at most 16 UTF-8 bytes"),
        "refused naming the bound of the field:\n{out}"
    );
}

/// A value that is neither a literal, a text field of a record nor `null` is refused
/// where it is written to an optional text field, by the rule every record's text
/// field keeps: the build cannot bound it.
#[test]
fn kotlin_refuses_a_value_nothing_bounds_written_to_an_optional_text_field() {
    let body = format!(
        r#"{MINE}
    <sce:var name="word" type="optional&lt;string&gt;" init="null"/>
    <sce:assign target="mine.title" expr="word"/>
    <sce:return expr="null"/>"#
    );
    let (ok, out, _) = run_kotlin(
        "generate",
        &[("algorithm_due_label.scxml", &on_due(&body))],
        "algorithm_due_label.scxml",
    );
    assert!(!ok, "an unbounded text is refused:\n{out}");
    assert!(
        out.contains("written to the string field `title`"),
        "refused at the write:\n{out}"
    );
}

// ── A statechart's call ──────────────────────────────────────────────────

/// A statechart that holds a record of `schema_due`, hands it to `Label` and takes the
/// optional text it answers into a variable bounded by `sce:capacity`.
const MACHINE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="optional_text_call">
  <sce:import kind="event-schema" src="schema_due.scxml" as="Due"/>
  <sce:import kind="algorithm" src="algorithm_due_label.scxml" as="Label"/>
  <datamodel>
    <data id="item" sce:type="record:Due">
      <sce:set name="id" expr="0"/>
      <sce:set name="due" expr="null"/>
      <sce:set name="title" expr="null"/>
      <sce:set name="status" expr="null"/>
    </data>
    <data id="label" sce:type="optional&lt;string&gt;" sce:capacity="16" expr="null"/>
  </datamodel>
  <state id="idle">
    <transition event="go" type="internal">
      <assign location="label" expr="Label(item)"/>
    </transition>
  </state>
</scxml>
"#;

/// The adapter of the call hands the algorithm a record whose optional text and day
/// are the machine's as they are and whose optional value of an enum crosses where it
/// is present, by the name of its variant, and takes the optional text it answers as
/// it is, held to the bound of the variable it is taken into.
#[test]
fn kotlin_adapts_a_call_that_hands_over_a_record_with_optional_fields() {
    let (ok, out, written) = run_kotlin(
        "generate",
        &[
            (
                "algorithm_due_label.scxml",
                &on_due(r#"<sce:return expr="item.title"/>"#),
            ),
            ("optional_text_call.scxml", MACHINE),
        ],
        "optional_text_call.scxml",
    );
    assert!(ok, "Kotlin generates the machine:\n{out}");
    holds_each(
        &written,
        &[
            "private fun call_Label__v(p0: OptionalTextCallDueRecord): String? {",
            "due = p0.due",
            "title = p0.title",
            // The enum crosses where it is present, a `when` over every variant.
            "status = (p0.status)?.let { scePresent -> when (scePresent) {",
            // The text it answers is held to the variable's bound where present.
            "?.let { scePresent -> com.sce.forge.runtime.SceChecked.bounded(scePresent, 16) }",
        ],
    );
}

/// The Interpreter's lowering holds the text an algorithm writes to a record's field of
/// `optional<string>` to the field's bound as it holds a text field's: a literal past it
/// is refused where it is written, `null` is the absent value and a literal that fits is
/// held to the bound where the algorithm runs.
#[test]
fn the_interpreter_holds_an_optional_text_field_to_its_bound() {
    let lowered = |body: &str| -> (bool, String, String) {
        let dir = tempdir().expect("tempdir");
        for name in SIBLINGS {
            std::fs::copy(fixtures().join(name), dir.path().join(name)).expect("copy a sibling");
        }
        std::fs::write(dir.path().join("algorithm_due_label.scxml"), on_due(body))
            .expect("write the algorithm");
        std::fs::write(dir.path().join("optional_text_call.scxml"), MACHINE)
            .expect("write the machine");
        let out_dir = tempdir().expect("tempdir");
        let out = Command::new(sce_codegen_bin())
            .arg("--workspace-root")
            .arg(repo_root())
            .arg("--error-format=json")
            .arg("lower")
            .arg(dir.path().join("optional_text_call.scxml"))
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
            String::from_utf8_lossy(&out.stderr).into_owned()
                + &String::from_utf8_lossy(&out.stdout),
            written,
        )
    };
    let fits = format!(
        r#"{MINE}
    <sce:assign target="mine.title" expr="'abc'"/>
    <sce:if cond="mine.title !== null">
      <sce:assign target="mine.title" expr="null"/>
    </sce:if>
    <sce:return expr="mine.title"/>"#
    );
    let (ok, out, written) = lowered(&fits);
    assert!(ok, "the Interpreter lowers a text that fits:\n{out}");
    assert!(
        written.contains("SceStatic.bounded("),
        "a literal is held to the bound of its field:\n{written}"
    );
    let past = format!(
        r#"{MINE}
    <sce:assign target="mine.title" expr="'a text that is longer than sixteen bytes'"/>
    <sce:return expr="null"/>"#
    );
    let (ok, out, _) = lowered(&past);
    assert!(!ok, "the Interpreter refuses a text past the bound:\n{out}");
    assert!(
        out.contains("which holds at most 16"),
        "refused naming the bound of the field:\n{out}"
    );
}

/// What a call crosses is named by the document that declares it: an optional value
/// of an enum or a byte string in a slot of an imported algorithm is not one a
/// statechart reads through yet, and is refused by name where the import is read, as
/// it was before the algorithm could spell one.
#[test]
fn kotlin_still_refuses_an_optional_enum_or_bytes_in_the_slot_of_a_call_by_name() {
    for (imports, written, named) in [
        (
            "  <sce:import kind=\"enum\" src=\"enum_item_status.scxml\" as=\"Status\"/>\n",
            "optional&lt;enum:Status&gt;",
            "optional<enum:Status>",
        ),
        ("", "optional&lt;bytes&gt;", "optional<bytes>"),
    ] {
        let algorithm = slots(imports, written, written, written);
        let machine = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="slot_call">
  <sce:import kind="algorithm" src="slot_probe.scxml" as="Probe"/>
  <datamodel>
    <data id="count" sce:type="int32" expr="0"/>
  </datamodel>
  <state id="idle">
    <transition event="go" type="internal">
      <assign location="count" expr="Probe(null) === null ? 0 : 1"/>
    </transition>
  </state>
</scxml>
"#;
        let (ok, out, _) = run_kotlin(
            "check",
            &[
                ("slot_probe.scxml", &algorithm),
                ("slot_call.scxml", machine),
            ],
            "slot_call.scxml",
        );
        assert!(!ok, "a call that crosses `{named}` is refused:\n{out}");
        assert!(
            out.contains(named),
            "refused naming the type of the slot:\n{out}"
        );
    }
}
