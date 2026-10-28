// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The Kotlin backend holds a record that may be absent, `optional<record:X>`
//! (docs/adr/0055, step 9): a `sce-static` statechart's variable is the record's own
//! data class over `?`, started at `null`; a whole record is put into it from a
//! payload, a record variable, a list's element and an algorithm's answer; a field of
//! it is read as `SceOptional.proven(x).f` where a test proves the record present
//! and written as the data class replaced by itself with that field changed; an
//! algorithm's parameter, return and local are of the same type; and a saved state
//! holds it as `null` or as the record.
//!
//! `scenarios/static_optional_record.json` replays the machine the build generates
//! (`StaticScenarioTest`), and its saved instance is restored by
//! `StaticDatamodelTest`; this file holds the text the generator writes, which is
//! what those two read.
//!
//! # What was wrong
//!
//! Measured in the code on 2026-10-28: the Kotlin backend refused a document that
//! declared an optional record, calls of an algorithm that took or returned one and
//! an algorithm that held one, by name, as every generated backend did.

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
/// record `Item`, the algorithm that answers an optional one, a second record
/// (`Day`) and the machine of the scenario.
const SIBLINGS: [&str; 4] = [
    "schema_item_pick.scxml",
    "algorithm_item_pick_first.scxml",
    "schema_day.scxml",
    "static_optional_record.scxml",
];

/// `sce-codegen generate -l kotlin <file>` in a directory holding the siblings and
/// `documents`, `file` being one of them or a sibling. Returns `(exit_ok, stderr +
/// stdout, the text of every file written)`.
fn generate_kotlin(documents: &[(&str, &str)], file: &str) -> (bool, String, String) {
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
        .arg("--error-format=json")
        .args(["generate", "-l", "kotlin"])
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

// ── The machine of the scenario ──────────────────────────────────────────

/// The fixture machine of `static_optional_record.json`: the variable is the data
/// class over `?` starting at `null`, a published one is read as it is, a test is
/// `!= null`, a field is read through the one function a proven read is spelled
/// with and written as the data class replaced with that field changed, and a list's
/// element is taken whole by the runtime's checked read.
#[test]
fn kotlin_generates_the_machine_that_holds_an_optional_record() {
    let (ok, out, written) = generate_kotlin(&[], "static_optional_record.scxml");
    assert!(ok, "Kotlin generates the machine:\n{out}");
    holds_each(
        &written,
        &[
            // The variable, started absent, and the property a host reads it by.
            "private var vPick: StaticOptionalRecordItemRecord? = null",
            "val pick: StaticOptionalRecordItemRecord? get() = vPick",
            // The test that proves the record present.
            "vPick != null",
            // A field read where it is proven, held to the bound of the text it is
            // taken into.
            "com.sce.forge.runtime.SceOptional.proven(vPick).title",
            "com.sce.forge.runtime.SceOptional.proven(vPick).count",
            // A field written: the record replaced with that field changed.
            "vPick = com.sce.forge.runtime.SceOptional.proven(vPick).copy(count = ",
            "vPick = com.sce.forge.runtime.SceOptional.proven(vPick).copy(title = ",
            // A list's element taken whole, and a record variable taken whole.
            "com.sce.forge.runtime.SceChecked.at(vItems, (1).toLong())",
            "vPick = vDraft",
            // The record type the variable is of is the one a record variable is of.
            "data class StaticOptionalRecordItemRecord(",
        ],
    );
}

/// What the machine saves of an optional record is `null` or the record's own saved
/// form, and what it restores is absent from `null` and the record otherwise, read as
/// a record variable is.
#[test]
fn kotlin_saves_an_optional_record_as_null_or_the_record() {
    let (ok, out, written) = generate_kotlin(&[], "static_optional_record.scxml");
    assert!(ok, "Kotlin generates the machine:\n{out}");
    holds_each(
        &written,
        &[
            "\"pick\" to SavedValues.optional(vPick) { it.toSaved() }",
            "\"found\" to SavedValues.optional(vFound) { it.toSaved() }",
            "SavedValues.optional(savedGrown.variable(\"pick\"), \"pick\") { e, w -> \
             StaticOptionalRecordItemRecord.fromSaved(e, w) }",
            // A record that is never absent is saved and read as it always was.
            "\"draft\" to vDraft.toSaved()",
            "StaticOptionalRecordItemRecord.fromSaved(savedGrown.variable(\"draft\"), \"draft\")",
        ],
    );
}

/// An algorithm that takes a list of records and answers an optional record is
/// called through an adapter that builds the algorithm's record where the answer is
/// present and leaves an absent one absent.
#[test]
fn kotlin_adapts_an_algorithm_that_answers_an_optional_record() {
    let (ok, out, written) = generate_kotlin(&[], "static_optional_record.scxml");
    assert!(ok, "Kotlin generates the machine:\n{out}");
    holds_each(
        &written,
        &[
            "private fun call_FirstAtLeast__v(p0: List<StaticOptionalRecordItemRecord>, p1: Int): \
             StaticOptionalRecordItemRecord? {",
            "return r?.let { StaticOptionalRecordItemRecord(id = it.id, title = it.title, count = \
             it.count) }",
            "vFound = call_FirstAtLeast__v(vItems, 5)",
        ],
    );
}

/// An optional record is handed to an algorithm by the name of the variable that
/// holds it, by a record variable (lifted) or by `null`: the first two are one
/// adapter whose parameter is the machine's own `T?`, which a `T` stands in for as it
/// is, and `null` another, so that a call never differs in what its adapter takes.
#[test]
fn kotlin_hands_an_optional_record_to_an_algorithm_by_the_name_that_holds_it() {
    let text = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="optional_record_call">
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
    <transition event="go" type="internal">
      <assign location="seen" expr="PickCount(pick, items)"/>
      <assign location="seen" expr="PickCount(null, items)"/>
      <assign location="seen" expr="PickCount(draft, items)"/>
    </transition>
  </state>
</scxml>
"#;
    let (ok, out, written) = generate_kotlin(
        &[
            ("algorithm_pick_count.scxml", &pick_count(PICK_COUNT_BODY)),
            ("probe.scxml", text),
        ],
        "probe.scxml",
    );
    assert!(ok, "Kotlin generates the machine:\n{out}");
    // The variable and the record variable are held by `v`, `null` by `n`: an
    // adapter each, whose first parameter is the machine's `T?`.
    for adapter in ["call_PickCount__vv", "call_PickCount__nv"] {
        let signature = format!(
            "private fun {adapter}(p0: OptionalRecordCallItemRecord?, p1: \
             List<OptionalRecordCallItemRecord>): Int {{"
        );
        holds_each(&written, &[signature.as_str()]);
    }
    holds_each(
        &written,
        &[
            // The algorithm's record is built where the argument is present.
            "p0?.let { SchemaItemPickPayload(id = it.id, title = it.title, count = it.count) }",
            // The three calls: the variable, `null` as the absent value, and the
            // record variable, lifted by the adapter.
            "call_PickCount__vv(vPick, vItems)",
            "call_PickCount__nv(null, vItems)",
            "call_PickCount__vv(vDraft, vItems)",
        ],
    );
}

// ── An algorithm holds one ───────────────────────────────────────────────

/// An algorithm that takes an optional record `x` and a list of records, keeps one of
/// them in a local `held` that starts absent, and answers the count of `x` where it is
/// present, or of the local, or -1. `body` is what it runs.
fn pick_count(body: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
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
    {body}
  </sce:body>
</scxml>
"#
    )
}

/// What [`pick_count`] runs in the tests that generate it: the first item of the list
/// is kept in the local while it is absent, the count of `x` is answered where `x` is
/// present, and the count of the local, written first, where it is.
const PICK_COUNT_BODY: &str = r#"<sce:foreach item="it" in="items">
      <sce:if cond="held === null">
        <sce:assign target="held" expr="it"/>
      </sce:if>
    </sce:foreach>
    <sce:if cond="x !== null">
      <sce:return expr="x.value.count"/>
    </sce:if>
    <sce:if cond="held !== null">
      <sce:assign target="held.value.count" expr="7"/>
      <sce:assign target="held.value.title" expr="'seven'"/>
      <sce:return expr="held.value.count"/>
    </sce:if>
    <sce:return expr="-1"/>"#;

/// A parameter, a local and a return of an optional record are the record's own
/// data class over `?`; `null` is the absent value, a test is `!= null`, a field is
/// read through the one function a proven read is spelled with and written as the
/// data class replaced with that field changed, a text field from a literal the
/// build bounds.
#[test]
fn kotlin_spells_an_algorithms_optional_record_in_its_slots_and_writes_its_fields() {
    let (ok, out, written) = generate_kotlin(
        &[("algorithm_pick_count.scxml", &pick_count(PICK_COUNT_BODY))],
        "algorithm_pick_count.scxml",
    );
    assert!(ok, "Kotlin generates the algorithm:\n{out}");
    holds_each(
        &written,
        &[
            // The parameter and the local, started absent.
            "x: SchemaItemPickPayload?",
            "var held: SchemaItemPickPayload? = null",
            // A record put into the local whole, by its name.
            "held = it",
            // The tests, and the proven reads.
            "held == null",
            "x != null",
            "return com.sce.forge.runtime.SceOptional.proven(x).count",
            // The writes: the data class replaced with each field changed.
            "held = com.sce.forge.runtime.SceOptional.proven(held).copy(count = 7)",
            "held = com.sce.forge.runtime.SceOptional.proven(held).copy(title = \"seven\")",
        ],
    );
}

/// An algorithm that answers an optional record is the type its slot is: the
/// scenario's `item_pick_first` generates with a list of records in and the record
/// that may be absent out.
#[test]
fn kotlin_generates_the_algorithm_of_the_scenario_that_answers_an_optional_record() {
    let (ok, out, written) = generate_kotlin(&[], "algorithm_item_pick_first.scxml");
    assert!(ok, "Kotlin generates the algorithm:\n{out}");
    holds_each(
        &written,
        &[
            "): SchemaItemPickPayload? {",
            "var found: SchemaItemPickPayload? = null",
            "found == null",
            "found = it",
            "return found",
        ],
    );
}

/// An algorithm writes a field of an optional record only where a test proves it
/// present, which the judge holds before Kotlin renders it: the write the renderer
/// spells is never one of a record that may be absent.
#[test]
fn kotlin_refuses_a_write_through_an_optional_record_nothing_proves_present() {
    let (ok, out, _) = generate_kotlin(
        &[(
            "algorithm_pick_count.scxml",
            &pick_count(
                r#"<sce:assign target="held.value.count" expr="7"/>
    <sce:return expr="-1"/>"#,
            ),
        )],
        "algorithm_pick_count.scxml",
    );
    assert!(!ok, "an unproven write is refused:\n{out}");
    assert!(
        out.contains("where no test proves `held` present"),
        "refused at the write:\n{out}"
    );
}

/// A record is put into an optional record local and returned as the optional record
/// the signature declares by its name: a bare record name is no expression, so each is
/// the local itself.
#[test]
fn kotlin_puts_and_returns_a_record_by_name_as_an_optional_record() {
    let text = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="first_item" version="1.0">
  <sce:import kind="event-schema" src="schema_item_pick.scxml" as="Item"/>
  <sce:signature>
    <sce:param name="items" type="list&lt;record:Item&gt;"/>
    <sce:return type="optional&lt;record:Item&gt;"/>
  </sce:signature>
  <sce:body>
    <sce:var name="held" type="optional&lt;record:Item&gt;" init="null"/>
    <sce:foreach item="it" in="items">
      <sce:assign target="held" expr="it"/>
      <sce:return expr="it"/>
    </sce:foreach>
    <sce:return expr="held"/>
  </sce:body>
</scxml>
"#;
    let (ok, out, written) = generate_kotlin(
        &[("algorithm_first_item.scxml", text)],
        "algorithm_first_item.scxml",
    );
    assert!(ok, "Kotlin generates the algorithm:\n{out}");
    holds_each(&written, &["held = it", "return it", "return held"]);
}
