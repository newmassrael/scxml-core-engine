// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! What generated C++ spells for a record that may be absent, `optional<record:X>`,
//! and for the optional elements an algorithm holds (docs/adr/0055, step 9): a
//! statechart's variable is a `std::optional` over the machine's own record, started
//! at `std::nullopt`; a record is taken whole into it from a payload, a record
//! variable, a list's element (the runtime's checked read) and an algorithm's answer;
//! a field of the present record is read as `(*x).f` and written as `(*x).f = e`
//! where a test proves it present; an algorithm holds one in a parameter, a local and
//! a return, a text, a value of an enum and a record are optional elements of its
//! slots, and a record it holds may have an optional text or enum field, crossing an
//! adapter by the declared name of the variant. A byte string is no optional element
//! of an algorithm's slot, since the algorithm holds it as a view, a buffer or a
//! vector by the slot, and stays refused by name.
//!
//! The scenario `static_optional_record.json` replays the machine against the
//! generated code; this file holds what the generator writes, which a scenario that
//! reads the machine back through its saved state does not see (the spelling of an
//! adapter, of a write through the present record, of an algorithm's slot).
//!
//! # What was wrong
//!
//! Measured in the code on 2026-10-28: C++ refused a statechart that declared an
//! optional record, an algorithm that took one, and an algorithm slot of an optional
//! text, enum or record, each as `generate/unsupported-feature`.

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

/// The documents of the fixture directory a probe imports, copied beside it.
const SIBLINGS: [&str; 6] = [
    "schema_item_pick.scxml",
    "algorithm_item_pick_first.scxml",
    "static_optional_record.scxml",
    "schema_day.scxml",
    "schema_due.scxml",
    "enum_item_status.scxml",
];

/// `sce-codegen generate -l cpp <file>` in a directory holding the siblings and
/// `documents`, `file` being one of them. Returns `(exit_ok, stderr + stdout, the
/// text of every file written)`.
fn generate(documents: &[(&str, &str)], file: &str) -> (bool, String, String) {
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
        .args(["generate", "-l", "cpp"])
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

/// `generate` of `file`, which must generate; the text of what was written.
fn generated(documents: &[(&str, &str)], file: &str) -> String {
    let (ok, out, written) = generate(documents, file);
    assert!(ok, "{file} generates for C++:\n{out}");
    written
}

/// Every `needle` is in `written`.
fn holds_each(written: &str, needles: &[&str]) {
    for needle in needles {
        assert!(
            written.contains(needle),
            "the generated C++ holds `{needle}`:\n{written}"
        );
    }
}

// ── The machine of the scenario ──────────────────────────────────────────

/// The fixture machine of `static_optional_record.json`: each optional record is a
/// `std::optional` over the machine's record that starts empty, a field of the
/// present one is read and written through `*`, a list's element is read by the
/// checked read of the runtime, and the saved state writes the variable as null or
/// as the record, through the record's own saved form.
#[test]
fn the_scenario_machine_holds_an_optional_record_in_a_std_optional() {
    let written = generated(&[], "static_optional_record.scxml");
    holds_each(
        &written,
        &[
            // The variable, started absent.
            "ItemRecord> v_pick = std::optional<",
            "ItemRecord> v_found = std::optional<",
            // Tested, and a field read where the test proves it present.
            "v_pick.has_value()",
            "(*v_pick).count",
            "(*v_pick).title",
            // A field written through the present record.
            "(*v_pick).count = ",
            "(*v_pick).title = ",
            // A list's element taken whole.
            "SCE::Forge::Checked::at(sce_failure_, v_items, 1)",
            // The saved form: null or the record, by the record's own functions.
            "savedWriteOptional(this->policy_.v_pick, ",
            "ItemRecordToSaved)",
            "savedReadOptional(grown.variable(\"pick\"), \"pick\", ",
            "ItemRecordFromSaved)",
            // The answer of an algorithm, made the machine's record where there is one.
            "SCE::Forge::mapPresent(r, [&](const SCE::Generated::SchemaItemPick::SchemaItemPickPayload &scePresent)",
        ],
    );
}

/// The algorithm of the scenario, `item_pick_first`: an optional record in its local
/// and its return, tested, assigned a record whole by its name and returned.
#[test]
fn an_algorithm_holds_an_optional_record_in_a_local_and_a_return() {
    let written = generated(&[], "algorithm_item_pick_first.scxml");
    holds_each(
        &written,
        &[
            "inline std::optional<SCE::Generated::SchemaItemPick::SchemaItemPickPayload> ",
            "std::optional<SCE::Generated::SchemaItemPick::SchemaItemPickPayload> found = std::nullopt;",
            "!found.has_value()",
            "found = it;",
            "return found;",
        ],
    );
}

// ── An algorithm's slots ─────────────────────────────────────────────────

/// An algorithm that takes `x`, an optional record, and keeps it in `held`: a field
/// of the present record is written through `*` where a test proves it present, a
/// text held to the bound its schema declares, and the record is read through the
/// same.
fn pick_bump() -> String {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="pick_bump" version="1.0">
  <sce:import kind="event-schema" src="schema_item_pick.scxml" as="Item"/>
  <sce:signature>
    <sce:param name="x" type="optional&lt;record:Item&gt;"/>
    <sce:return type="int32"/>
  </sce:signature>
  <sce:body>
    <sce:var name="held" type="optional&lt;record:Item&gt;" init="null"/>
    <sce:assign target="held" expr="x"/>
    <sce:if cond="held !== null">
      <sce:assign target="held.value.count" expr="7"/>
      <sce:assign target="held.value.title" expr="'bumped'"/>
      <sce:return expr="held.value.count"/>
    </sce:if>
    <sce:return expr="-1"/>
  </sce:body>
</scxml>
"#
    .to_string()
}

#[test]
fn a_field_of_an_algorithms_optional_record_is_written_through_the_present_record() {
    let written = generated(&[("pick_bump.scxml", &pick_bump())], "pick_bump.scxml");
    holds_each(
        &written,
        &[
            // The parameter and the local, by value.
            "std::optional<SCE::Generated::SchemaItemPick::SchemaItemPickPayload> x",
            "std::optional<SCE::Generated::SchemaItemPick::SchemaItemPickPayload> held = std::nullopt;",
            "held = x;",
            "held.has_value()",
            // A number and a text written through the present record, the text from
            // a literal the validator bounds.
            "(*held).count = 7;",
            "(*held).title = \"bumped\";",
            // And read the same way.
            "(*held).count",
        ],
    );
}

/// A write of a text to a field of the present record that is past the bound the
/// schema declares is refused where it is written, as a record's own text field is.
#[test]
fn a_text_written_through_the_present_record_is_held_to_its_bound() {
    let text = pick_bump().replace(
        "'bumped'",
        "'a text longer than the sixteen bytes the field holds'",
    );
    let (ok, out, _) = generate(&[("pick_bump.scxml", &text)], "pick_bump.scxml");
    assert!(!ok, "a text past the bound is refused:\n{out}");
    assert!(
        out.contains("at most 16 UTF-8 bytes"),
        "refused naming the bound of the field:\n{out}"
    );
}

/// An algorithm of no body to speak of whose parameter, return and local are declared
/// `param`, `ret` and `local`, importing what they name.
fn slots(param: &str, ret: &str, local: &str) -> String {
    let mut imports = String::new();
    if [param, ret, local]
        .iter()
        .any(|t| t.contains("enum:Status"))
    {
        imports.push_str(
            "  <sce:import kind=\"enum\" src=\"enum_item_status.scxml\" as=\"Status\"/>\n",
        );
    }
    if [param, ret, local]
        .iter()
        .any(|t| t.contains("record:Item"))
    {
        imports.push_str(
            "  <sce:import kind=\"event-schema\" src=\"schema_item_pick.scxml\" as=\"Item\"/>\n",
        );
    }
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

/// A text, a value of an enum and a record are each an optional element of an
/// algorithm's parameter, return and local, spelled `std::optional` over the type the
/// element is held in.
#[test]
fn an_algorithm_holds_an_optional_text_enum_and_record_in_every_slot() {
    for (written, spelled) in [
        ("optional&lt;string&gt;", "std::optional<std::string>"),
        (
            "optional&lt;enum:Status&gt;",
            "std::optional<SCE::Generated::EnumItemStatus::EnumItemStatus>",
        ),
        (
            "optional&lt;record:Item&gt;",
            "std::optional<SCE::Generated::SchemaItemPick::SchemaItemPickPayload>",
        ),
    ] {
        let text = slots(written, written, written);
        let out = generated(&[("slot_probe.scxml", &text)], "slot_probe.scxml");
        let needles = [
            format!("{spelled} given"),
            format!("{spelled} held = std::nullopt;"),
            format!("inline {spelled} "),
        ];
        let needles: Vec<&str> = needles.iter().map(String::as_str).collect();
        holds_each(&out, &needles);
    }
}

/// A byte string is no optional element of an algorithm's slot: each slot refuses it
/// by name, where it is rendered.
#[test]
fn an_algorithm_refuses_an_optional_byte_string_in_a_slot_by_name() {
    let bytes = "optional&lt;bytes&gt;";
    let number = "optional&lt;int64&gt;";
    for (param, ret, local) in [
        (bytes, number, number),
        (number, bytes, number),
        (number, number, bytes),
    ] {
        let text = slots(param, ret, local)
            .replace("<sce:return expr=\"held\"/>", "<sce:return expr=\"null\"/>");
        let (ok, out, _) = generate(&[("slot_probe.scxml", &text)], "slot_probe.scxml");
        assert!(
            !ok,
            "`{bytes}` is refused on a slot of the algorithm:\n{out}"
        );
        assert!(
            out.contains("generate/unsupported-feature")
                && out.contains("an optional value has no")
                && out.contains("lowering yet"),
            "refused by name:\n{out}"
        );
    }
}

// ── What crosses an adapter ──────────────────────────────────────────────

/// A statechart that imports the algorithm `due_retitle` as `Retitle`, the schema `Due`
/// and the enum `Status`, holds a record `d` of the schema and an optional text
/// `shown`, and calls the algorithm on `go`.
fn due_machine() -> String {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="due_probe">
  <sce:import kind="enum" src="enum_item_status.scxml" as="Status"/>
  <sce:import kind="event-schema" src="schema_due.scxml" as="Due"/>
  <sce:import kind="algorithm" src="algorithm_due_retitle.scxml" as="Retitle"/>
  <datamodel>
    <data id="d" sce:type="record:Due">
      <sce:set name="id" expr="1"/>
      <sce:set name="due" expr="null"/>
      <sce:set name="title" expr="null"/>
      <sce:set name="status" expr="null"/>
    </data>
    <data id="plain" sce:type="enum:Status" expr="Status.open"/>
    <data id="shown" sce:type="optional&lt;string&gt;" sce:capacity="16" expr="null"/>
  </datamodel>
  <state id="idle">
    <transition event="go">
      <assign location="shown" expr="Retitle(d)"/>
    </transition>
  </state>
</scxml>
"#
    .to_string()
}

/// An algorithm that takes a record whose text and enum fields may be absent, writes
/// its text (a literal, which the validator bounds, and `null`), and answers it.
fn due_retitle() -> String {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="due_retitle" version="1.0">
  <sce:import kind="event-schema" src="schema_due.scxml" as="Due"/>
  <sce:import kind="enum" src="enum_item_status.scxml" as="Status"/>
  <sce:signature>
    <sce:param name="d" type="record:Due"/>
    <sce:return type="optional&lt;string&gt;"/>
  </sce:signature>
  <sce:body>
    <sce:assign target="d.title" expr="'soon'"/>
    <sce:if cond="d.status !== null">
      <sce:assign target="d.title" expr="null"/>
    </sce:if>
    <sce:return expr="d.title"/>
  </sce:body>
</scxml>
"#
    .to_string()
}

/// A record with an optional text and an optional enum field is held by an algorithm
/// whole: its text is written from a bounded literal or made absent, the enum field is
/// tested, and the answer is an optional text.
#[test]
fn an_algorithm_writes_the_optional_text_field_of_a_record_it_holds() {
    let written = generated(
        &[("algorithm_due_retitle.scxml", &due_retitle())],
        "algorithm_due_retitle.scxml",
    );
    holds_each(
        &written,
        &[
            "d.title = \"soon\";",
            "d.title = std::nullopt;",
            "d.status.has_value()",
            "return d.title;",
        ],
    );
}

/// The record crosses the adapter field by field: an optional text as it is, and an
/// optional enum by the declared name of its variant, mapped where it is present
/// (`mapPresent`) and absent where it is absent; the answer is an optional text.
#[test]
fn an_optional_enum_field_crosses_an_adapter_by_the_declared_name_of_its_variant() {
    let written = generated(
        &[
            ("algorithm_due_retitle.scxml", &due_retitle()),
            ("due_probe.scxml", &due_machine()),
        ],
        "due_probe.scxml",
    );
    holds_each(
        &written,
        &[
            "std::optional<std::string> call_Retitle__v(",
            "/*title=*/p0.title",
            "SCE::Forge::mapPresent(p0.status, [&](const ",
            "StatusEnum &scePresent)",
        ],
    );
}

/// An algorithm that takes an optional record of the schema `Day` and answers its
/// year where one is present.
fn year_of() -> String {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="year_of" version="1.0">
  <sce:import kind="event-schema" src="schema_day.scxml" as="Day"/>
  <sce:signature>
    <sce:param name="d" type="optional&lt;record:Day&gt;"/>
    <sce:return type="int32"/>
  </sce:signature>
  <sce:body>
    <sce:if cond="d !== null">
      <sce:return expr="d.value.year"/>
    </sce:if>
    <sce:return expr="-1"/>
  </sce:body>
</scxml>
"#
    .to_string()
}

/// A statechart that calls `year_of` with each way an optional record is handed over:
/// the payload of the event `day.picked` (the schema's own), a variable that may be
/// absent, a record variable (a present value), the literal `null` and the item of a
/// `<foreach>`.
fn year_machine() -> String {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="year_probe">
  <sce:import kind="event-schema" src="schema_day.scxml" as="Day"/>
  <sce:import kind="algorithm" src="algorithm_year_of.scxml" as="YearOf"/>
  <datamodel>
    <data id="maybe" sce:type="optional&lt;record:Day&gt;" expr="null"/>
    <data id="day" sce:type="record:Day">
      <sce:set name="year" expr="2026"/>
      <sce:set name="month" expr="9"/>
      <sce:set name="dayOfMonth" expr="24"/>
    </data>
    <data id="days" sce:type="list&lt;record:Day&gt;" sce:capacity="3"/>
    <data id="seen" sce:type="int32" expr="0"/>
  </datamodel>
  <state id="idle">
    <transition event="day.picked">
      <assign location="seen" expr="YearOf(_event.data)"/>
    </transition>
    <transition event="probe">
      <assign location="seen" expr="YearOf(maybe)"/>
      <assign location="seen" expr="YearOf(day)"/>
      <assign location="seen" expr="YearOf(null)"/>
      <foreach array="days" item="d">
        <assign location="seen" expr="YearOf(d)"/>
      </foreach>
    </transition>
  </state>
</scxml>
"#
    .to_string()
}

/// Each way an optional record is handed to an algorithm has its adapter. A variable
/// that may be absent, a record variable and `null` are held as the machine's own
/// `std::optional` (a record converts to one, and `null` is `std::nullopt`) and mapped
/// into the algorithm's record where one is present; the payload and a loop's item are
/// records, lifted into the present value.
#[test]
fn each_way_an_optional_record_is_handed_to_an_algorithm_has_its_adapter() {
    let written = generated(
        &[
            ("algorithm_year_of.scxml", &year_of()),
            ("year_probe.scxml", &year_machine()),
        ],
        "year_probe.scxml",
    );
    holds_each(
        &written,
        &[
            // The variable, the record variable and `null`.
            "call_YearOf__v(v_maybe)",
            "call_YearOf__v(v_day)",
            "call_YearOf__n(std::nullopt)",
            "SCE::Forge::mapPresent(p0, [&](const ",
            "{/*year=*/scePresent.year, /*month=*/scePresent.month, /*dayOfMonth=*/scePresent.dayOfMonth}",
            // The payload and the loop's item, lifted.
            "call_YearOf__p(",
            "call_YearOf__i(",
            "std::optional<SCE::Generated::SchemaDay::SchemaDayPayload>(SCE::Generated::SchemaDay::SchemaDayPayload{/*year=*/p0.year, /*month=*/p0.month, /*dayOfMonth=*/p0.dayOfMonth})",
        ],
    );
}
