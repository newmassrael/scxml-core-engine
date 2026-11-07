// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A Kotlin `sce-static` machine reads the payload of an event whose schema
//! holds a composition, at any depth (docs/adr/0086, section 6), as the Rust
//! machine does: its payload class holds each field in the Kotlin row of
//! section 7.1, each record it names is declared once, the lift reads each
//! field by one runtime call per constructor (section 4), the inject seam
//! writes each as its JSON (section 3.1), and the statechart's reads lower to
//! the machine's own types.
//!
//! # What is held
//!
//! * The payload class, the record a list of it holds, and the enum a list and
//!   a record hold, each in the machine's own type.
//! * The lift: a list of records that hold a list, a list of lists, an optional
//!   element, a list that may be absent, a top-level text held to its bound,
//!   and the defaults of the list the second revision added.
//! * The inject seam: the payload it is handed, written as the JSON a lift
//!   reads back.
//! * The reads: `<foreach>` over a list the payload holds and over a list an
//!   enclosing loop's item holds, an index into a list of lists, a field of an
//!   element read through its index.
//! * A payload that holds a union stays refused by name on Kotlin, which holds
//!   no union yet (docs/adr/0092, section 7).

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::{compile_scxml_lang_typed, find_template_dir_for};
use tempfile::tempdir;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

fn fixtures() -> PathBuf {
    repo_root().join("sce-build/tests/fixtures/static_datamodel")
}

/// The documents a reader of `batch.sent` imports, copied beside it.
const SIBLINGS: [&str; 3] = [
    "enum_signal_level.scxml",
    "event_batch_item.scxml",
    "event_batch.scxml",
];

/// A reader of `batch.sent` that measures a list at each depth, walks the
/// payload's `items` with an index and each item's `tags`, walks the payload's
/// list of lists and each of its lists, reads `grid[1][0]` and an element that
/// may be absent.
const READER: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="batch_reader">
  <sce:import kind="event-schema" src="event_batch.scxml" as="Batch"/>
  <sce:import kind="event-schema" src="event_batch_item.scxml" as="Item"/>
  <sce:import kind="enum" src="enum_signal_level.scxml" as="SignalLevel"/>
  <datamodel>
    <data id="seen" sce:type="uint32" expr="0" sce:direction="out"/>
    <data id="cell" sce:type="uint8" expr="0" sce:direction="out"/>
    <data id="first_maybe" sce:type="optional&lt;uint16&gt;" expr="null"/>
  </datamodel>
  <state id="idle">
    <transition event="batch.sent" target="idle">
      <assign location="seen"
              expr="len(_event.data.items) + len(_event.data.grid) + len(_event.data.levels)"/>
      <foreach array="_event.data.items" item="row" index="i">
        <assign location="seen" expr="seen + len(row.tags) + len(_event.data.items[i].tags)"/>
        <foreach array="row.tags" item="word">
          <assign location="seen" expr="seen + 1"/>
        </foreach>
      </foreach>
      <foreach array="_event.data.grid" item="cells">
        <foreach array="cells" item="n">
          <assign location="cell" expr="n"/>
        </foreach>
      </foreach>
      <if cond="len(_event.data.grid) &gt; 1 &amp;&amp; len(_event.data.grid[1]) &gt; 0">
        <assign location="cell" expr="_event.data.grid[1][0]"/>
      </if>
      <if cond="len(_event.data.maybe) &gt; 0">
        <assign location="first_maybe" expr="_event.data.maybe[0]"/>
      </if>
    </transition>
  </state>
</scxml>
"#;

/// The Kotlin machine of `document`, written as `name` beside the batch
/// schemas.
fn kotlin_machine(name: &str, document: &str) -> String {
    let dir = tempdir().expect("tempdir");
    for sibling in SIBLINGS {
        std::fs::copy(fixtures().join(sibling), dir.path().join(sibling)).expect("copy a sibling");
    }
    let path = dir.path().join(name);
    std::fs::write(&path, document).expect("write the document");
    let language = sce_build::generator::Language::Kotlin;
    let output = compile_scxml_lang_typed(
        path.to_str().expect("a utf-8 path"),
        &find_template_dir_for(language),
        language,
    )
    .unwrap_or_else(|e| panic!("Kotlin lowers {name}: {e:?}"));
    let names: Vec<String> = output.files.iter().map(|(name, _)| name.clone()).collect();
    output
        .files
        .into_iter()
        .find(|(name, _)| name.ends_with("Sm.kt"))
        .map(|(_, code)| code)
        .unwrap_or_else(|| panic!("the machine's Kotlin is generated among {names:?}"))
}

/// The Kotlin machine of [`READER`].
fn reader_machine() -> String {
    kotlin_machine("batch_reader.scxml", READER)
}

/// `code` holds each of `lines`.
fn holds(code: &str, lines: &[&str]) {
    for line in lines {
        assert!(code.contains(line), "expected `{line}` in:\n{code}");
    }
}

/// Pinned by `KotlinComposed::record_type`, `KotlinComposed::list_type`,
/// `KotlinComposed::optional_type`, `KotlinComposed::text_type` and
/// `KotlinComposed::bytes_type` (`sce-build/src/forge/composed_lowering.rs`),
/// by `kotlin_composed_payload` (`sce-build/src/forge/generator.rs`), and by the
/// declaration of each record a payload names (`declare_composed_payload`,
/// `sce-build/src/forge/static_lowering.rs`).
#[test]
fn the_payload_is_held_in_the_machine_types_each_record_declared_once() {
    let code = reader_machine();
    holds(
        &code,
        &[
            "data class BatchReaderBatchSentPayload(val items: List<BatchReaderItemRecord>, \
             val grid: List<List<UByte>>, val levels: List<BatchReaderSignalLevelEnum>, \
             val maybe: List<UShort?>, val spare: List<UByte>?, val note: String, \
             val late: List<UByte>)",
            "data class BatchReaderItemRecord(val id: UInt, val amount: Long, val label: String, \
             val level: BatchReaderSignalLevelEnum, val due: Long?, val blob: ByteArray, \
             val tags: List<String>)",
        ],
    );
    assert_eq!(
        code.matches("data class BatchReaderItemRecord(").count(),
        1,
        "a record is declared once:\n{code}"
    );
}

/// Pinned by `KotlinComposed::field_checks` and `kotlin_init_block`
/// (`sce-build/src/forge/composed_lowering.rs`), the `class_body`
/// `kotlin_composed_payload` gives the payload class
/// (`sce-build/src/forge/generator.rs`) and the checks `declare_composed`
/// hands each record (`sce-build/src/forge/static_lowering.rs`): a value a
/// host makes, the payload it hands the inject seam and each record it builds
/// for it, is refused in its class's `init`, where it is made, at the first
/// value past a bound its type declares at any depth (docs/adr/0086, section
/// 5.4).
#[test]
fn a_value_a_host_makes_is_held_to_every_bound_where_it_is_made() {
    let code = reader_machine();
    let b = "com.sce.runtime.ValueBounds";
    holds(
        &code,
        &[
            "val late: List<UByte>) {\n    init {\n",
            &format!("        {b}.list(\"items\", items, 3) {{ _, _ -> }}\n"),
            &format!(
                "        {b}.list(\"grid\", grid, 2) {{ scePath0, sceValue0 -> \
                 {b}.list(scePath0, sceValue0, 2) {{ _, _ -> }} }}\n"
            ),
            &format!("        {b}.list(\"maybe\", maybe, 2) {{ _, _ -> }}\n"),
            &format!(
                "        {b}.optional(\"spare\", spare) {{ scePath0, sceValue0 -> \
                 {b}.list(scePath0, sceValue0, 2) {{ _, _ -> }} }}\n"
            ),
            &format!("        {b}.text(\"note\", note, 4)\n"),
            &format!("        {b}.list(\"late\", late, 2) {{ _, _ -> }}\n"),
            "val tags: List<String>) {\n    init {\n",
            &format!("        {b}.text(\"label\", label, 8)\n"),
            &format!("        {b}.bytes(\"blob\", blob, 4)\n"),
            &format!(
                "        {b}.list(\"tags\", tags, 4) {{ scePath0, sceValue0 -> \
                 {b}.text(scePath0, sceValue0, 8) }}\n"
            ),
        ],
    );
}

/// Pinned by the fit the judge asks of a whole copy (`Judge`,
/// `sce-build/src/forge/static_datamodel.rs`) before any backend lowers it: a
/// list that may hold more than the variable it is taken into is refused on
/// Kotlin where the document is checked, so the copy Kotlin writes as the list
/// itself never puts a list past its bound into a variable.
#[test]
fn a_copy_that_may_pass_the_variables_bound_is_refused_on_kotlin() {
    let document = READER.replace(
        "<data id=\"seen\"",
        "<data id=\"small\" sce:type=\"list&lt;uint8&gt;[1]\"/>\n    <data id=\"seen\"",
    );
    let document = document.replace(
        "<transition event=\"batch.sent\" target=\"idle\">",
        "<transition event=\"batch.sent\" target=\"idle\">\n      \
         <assign location=\"small\" expr=\"_event.data.late\"/>",
    );
    let (ok, out) = run(
        &["check", "-l", "kotlin"],
        &fixtures(),
        &SIBLINGS,
        "batch_reader.scxml",
        &document,
    );
    assert!(!ok, "a list of 2 may not fit a variable of 1:\n{out}");
    assert!(out.contains("scxml/static-datamodel-rule"), "{out}");
    assert!(
        out.contains("which may hold more than the list<uint8>[1] expected there"),
        "{out}"
    );
}

/// Pinned by `KotlinComposed::read_list`, `KotlinComposed::read_record`,
/// `KotlinComposed::read_text`, `KotlinComposed::read_optional` and
/// `KotlinComposed::read_enum`, and by `composed_defaults_wire`
/// (`sce-build/src/forge/schema_revision.rs`): each field read by one runtime
/// call per constructor, a text held to its bound at the top level too, a list
/// the second revision added read as empty.
#[test]
fn the_lift_reads_each_field_by_its_type_at_every_depth() {
    let code = reader_machine();
    holds(
        &code,
        &[
            r#"val fields = EventPayload.decode(metadata.data, "{\"late\":[]}")"#,
            r#"fields.field("items").let { it.list(3) { it.record(null) { BatchReaderItemRecord(id = it.field("id").let { it.uint32() }, amount = it.field("amount").let { it.int64() }, label = it.field("label").let { it.string(8) }, "#,
            r#"level = it.field("level").let { when (val sceName = it.name()) { "#,
            r#"else -> throw it.unknownVariant(sceName, "SignalLevel") } }, "#,
            r#"due = it.field("due").let { it.optional { it.int64() } }, "#,
            r#"blob = it.field("blob").let { it.bytes(4) }, tags = it.field("tags").let { it.list(4) { it.string(8) } }) } } }"#,
            r#"fields.field("grid").let { it.list(2) { it.list(2) { it.uint8() } } }"#,
            r#"fields.field("maybe").let { it.list(2) { it.optional { it.uint16() } } }"#,
            r#"fields.field("spare").let { it.optional { it.list(2) { it.uint8() } } }"#,
            r#"fields.field("note").let { it.string(4) }"#,
        ],
    );
}

/// Pinned by `KotlinComposed::write_list`, `KotlinComposed::write_record`,
/// `KotlinComposed::write_optional` and `KotlinComposed::write_enum`, and by
/// the prelude `kotlin_composed_payload` gives the inject seam: the seam builds
/// the payload it is handed and writes it as the JSON a lift reads back.
#[test]
fn the_inject_seam_writes_each_field_as_its_json() {
    let code = reader_machine();
    let p = "com.sce.runtime.EventPayload";
    holds(
        &code,
        &[
            "val scePayload = BatchReaderBatchSentPayload(items, grid, levels, maybe, spare, note, late)",
            "typedPayload = scePayload,",
            &format!(
                "data = {p}.recordJson(mapOf<String, Any?>(\"items\" to {p}.listJson(scePayload.items) {{ \
                 {p}.recordJson(mapOf<String, Any?>(\"id\" to {p}.valueJson(it.id), "
            ),
            &format!("\"level\" to {p}.valueJson(it.level.declaredName), "),
            &format!("\"due\" to ((it.due)?.let {{ {p}.valueJson(it) }} ?: {p}.valueJson(null)), "),
            &format!(
                "\"grid\" to {p}.listJson(scePayload.grid) {{ {p}.listJson(it) {{ {p}.valueJson(it) }} }}"
            ),
            &format!(
                "\"spare\" to ((scePayload.spare)?.let {{ {p}.listJson(it) {{ {p}.valueJson(it) }} }} \
                 ?: {p}.valueJson(null))"
            ),
            &format!("\"note\" to {p}.valueJson(scePayload.note)"),
        ],
    );
}

/// Pinned by the loop over a list a place holds (`walked_place`,
/// `sce-build/src/forge/static_lowering.rs`), `KotlinTarget::foreach_loop`, and
/// the runtime's checked read of a list at an index, which takes a list of any
/// element.
#[test]
fn the_reads_lower_at_any_depth() {
    let code = reader_machine();
    holds(
        &code,
        &[
            "for ((sce_position_of_i, row) in pendingBatchSentPayload!!.items.withIndex())",
            "for (word in row.tags)",
            "for (cells in pendingBatchSentPayload!!.grid)",
            "for (n in cells)",
            "com.sce.forge.runtime.SceChecked.at(pendingBatchSentPayload!!.items, ",
            "com.sce.forge.runtime.SceChecked.at(com.sce.forge.runtime.SceChecked.at(pendingBatchSentPayload!!.grid, ",
            "com.sce.forge.runtime.SceChecked.at(pendingBatchSentPayload!!.maybe, ",
        ],
    );
}

/// Pinned by `whole_list_copy` (`sce-build/src/forge/static_lowering.rs`),
/// `copier` and `KotlinComposed::copy_whole`
/// (`sce-build/src/forge/composed_lowering.rs`), and by `ComposedLowering::of`'s
/// `copies`: a list a payload holds is taken whole into a list variable of a
/// larger bound, one Kotlin type, so the copy is the list itself.
#[test]
fn a_list_is_taken_whole_into_a_variable_whose_type_it_fits() {
    let text =
        std::fs::read_to_string(fixtures().join("static_composed_payload.scxml")).expect("fixture");
    let code = kotlin_machine("static_composed_payload.scxml", &text);
    holds(
        &code,
        &[
            "private var vLate: List<UByte>",
            "vLate = pendingBatchSentPayload!!.late",
        ],
    );
    let (ok, out) = run(
        &["check", "-l", "kotlin"],
        &fixtures(),
        &SIBLINGS,
        "static_composed_payload.scxml",
        &text,
    );
    assert!(ok, "Kotlin lowers the fixture:\n{out}");
}

/// `sce-codegen <args> <file>` over `document` written as `name` in a
/// directory that holds `siblings` of `from`, and whether it succeeded, with
/// what it said.
fn run(
    args: &[&str],
    from: &Path,
    siblings: &[&str],
    name: &str,
    document: &str,
) -> (bool, String) {
    let dir = tempdir().expect("tempdir");
    for sibling in siblings {
        std::fs::copy(from.join(sibling), dir.path().join(sibling)).expect("copy a sibling");
    }
    std::fs::write(dir.path().join(name), document).expect("write the document");
    let out = Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(args)
        .arg(dir.path().join(name))
        .output()
        .expect("invoke sce-codegen");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout),
    )
}

/// Pinned by `ComposedLowering::of` (`sce-build/src/forge/event_schema_check.rs`):
/// Kotlin takes the read, and a backend that does not lower it refuses it by
/// its own name.
#[test]
fn kotlin_takes_the_read_and_another_backend_refuses_it_by_name() {
    let (ok, out) = run(
        &["check", "-l", "kotlin"],
        &fixtures(),
        &SIBLINGS,
        "batch_reader.scxml",
        READER,
    );
    assert!(ok, "Kotlin reads a composed payload:\n{out}");
    let (ok, out) = run(
        &["check", "-l", "python"],
        &fixtures(),
        &SIBLINGS,
        "batch_reader.scxml",
        READER,
    );
    assert!(!ok, "Python does not read a composed payload yet:\n{out}");
    assert!(out.contains("generate/unsupported-feature"), "{out}");
    assert!(
        out.contains("a statechart's read of `_event.data` has no python lowering of its list"),
        "{out}"
    );
}

/// A reader of `shapes.sent` that tests the tag of the payload's `one`,
/// importing the tag's enum, the event and the records its cases hold.
const SHAPES_READER: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="shapes_reader">
  <sce:import kind="enum" src="shape_kind.scxml" as="Kind"/>
  <sce:import kind="event-schema" src="event_shapes.scxml" as="Shapes"/>
  <sce:import kind="event-schema" src="shape_radius.scxml" as="Radius"/>
  <sce:import kind="event-schema" src="shape_rect.scxml" as="Rect"/>
  <datamodel>
    <data id="seen" sce:type="uint32" expr="0"/>
  </datamodel>
  <state id="idle">
    <transition event="shapes.sent" target="idle">
      <if cond="_event.data.one.kind === Kind.circle">
        <assign location="seen" expr="seen + 1"/>
      </if>
    </transition>
  </state>
</scxml>
"#;

/// The union fixtures a reader of `shapes.sent` imports.
const SHAPES: [&str; 5] = [
    "shape_kind.scxml",
    "shape_radius.scxml",
    "shape_rect.scxml",
    "union_shape.scxml",
    "event_shapes.scxml",
];

/// Pinned by the union arm of the gate's read (`first_unlowered_composed_use`,
/// `sce-build/src/forge/event_schema_check.rs`) and by `union_reached`
/// (`sce-build/src/forge/composed_lowering.rs`): Kotlin reads a composed
/// payload, holds no union, and so refuses a read of a payload that holds one
/// by its own name, naming the field and the union; Rust reads it.
#[test]
fn a_payload_that_holds_a_union_stays_refused_by_name_on_kotlin() {
    let unions = repo_root().join("sce-build/tests/fixtures/union");
    let (ok, out) = run(
        &["check", "-l", "kotlin"],
        &unions,
        &SHAPES,
        "shapes_reader.scxml",
        SHAPES_READER,
    );
    assert!(!ok, "Kotlin holds no union yet:\n{out}");
    for part in [
        "generate/unsupported-feature",
        "the field `one` holds `union:Shape`",
        "a statechart's read of `_event.data` has no kotlin lowering of its union",
    ] {
        assert!(out.contains(part), "expected `{part}` in:\n{out}");
    }
    let (ok, out) = run(
        &["check", "-l", "rust"],
        &unions,
        &SHAPES,
        "shapes_reader.scxml",
        SHAPES_READER,
    );
    assert!(ok, "Rust reads a payload that holds a union:\n{out}");
}
