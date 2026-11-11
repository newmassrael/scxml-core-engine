// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A Go `sce-static` machine reads the payload of an event whose schema holds
//! a composition, at any depth (docs/adr/0086, section 6), and sends a
//! composed value by its name (section 5.2), as the Rust and Kotlin machines
//! do: its payload struct holds each field in the Go row of section 7.1, each
//! record it names is declared once with a constructor a host builds one by,
//! the lift reads each field by one runtime call per constructor (section 4),
//! the inject seam holds what a host hands it to every bound its type declares
//! before it writes it as its JSON (sections 3.1 and 5.4), and the
//! statechart's reads, copies and sends lower to the machine's own types.
//!
//! # What is held
//!
//! * The payload struct, the record a list of it holds, and the enum a list
//!   and a record hold, each in the machine's own type.
//! * The lift: a list of records that hold a list, a list of lists, an
//!   optional element, a list that may be absent, a top-level text held to its
//!   bound, and the defaults of the list the second revision added.
//! * The inject seam: the checks it runs over what it is handed, and the
//!   payload it builds, written as the JSON a lift reads back.
//! * The reads: `<foreach>` over a list the payload holds and over a list an
//!   enclosing loop's item holds, an index into a list of the payload.
//! * A list taken whole into a variable, as a slice of its own.
//! * A `<send>` that names each composed field's place, each written as the
//!   value a send holds its params in, and one that gives a field of the first
//!   revision no value, refused.
//! * A payload that holds a union is read on Go, which holds a union in a
//!   value struct of its own (docs/adr/0092, section 7).

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
const SIBLINGS: [&str; 4] = [
    "enum_signal_level.scxml",
    "event_batch_item.scxml",
    "event_batch.scxml",
    "event_counted.scxml",
];

/// A reader of `batch.sent` that measures a list at each depth, walks the
/// payload's `items` with an index and each item's `tags`, walks the payload's
/// list of lists and each of its lists, and reads an element that may be
/// absent.
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
      <if cond="len(_event.data.maybe) &gt; 0">
        <assign location="first_maybe" expr="_event.data.maybe[0]"/>
      </if>
    </transition>
  </state>
</scxml>
"#;

/// The Go machine of `document`, written as `name` beside the batch schemas.
fn go_machine(name: &str, document: &str) -> String {
    let dir = tempdir().expect("tempdir");
    for sibling in SIBLINGS {
        std::fs::copy(fixtures().join(sibling), dir.path().join(sibling)).expect("copy a sibling");
    }
    let path = dir.path().join(name);
    std::fs::write(&path, document).expect("write the document");
    let language = sce_build::generator::Language::Go;
    let output = compile_scxml_lang_typed(
        path.to_str().expect("a utf-8 path"),
        &find_template_dir_for(language),
        language,
    )
    .unwrap_or_else(|e| panic!("Go lowers {name}: {e:?}"));
    let names: Vec<String> = output.files.iter().map(|(name, _)| name.clone()).collect();
    output
        .files
        .into_iter()
        .find(|(name, _)| name.ends_with("_sm.go"))
        .map(|(_, code)| code)
        .unwrap_or_else(|| panic!("the machine's Go is generated among {names:?}"))
}

/// The Go machine of [`READER`].
fn reader_machine() -> String {
    go_machine("batch_reader.scxml", READER)
}

/// The Go machine of the fixture `name`.
fn fixture_machine(name: &str) -> String {
    let text = std::fs::read_to_string(fixtures().join(name)).expect("fixture");
    go_machine(name, &text)
}

/// `code` holds each of `lines`.
fn holds(code: &str, lines: &[&str]) {
    for line in lines {
        assert!(code.contains(line), "expected `{line}` in:\n{code}");
    }
}

/// Pinned by `GoComposed::record_type`, `GoComposed::list_type`,
/// `GoComposed::optional_type`, `GoComposed::text_type`,
/// `GoComposed::bytes_type` and `GoComposed::record_def`
/// (`sce-build/src/forge/composed_lowering/go.rs`), by `go_composed_payload`
/// (`sce-build/src/forge/generator.rs`), and by the declaration of each record
/// a payload names (`declare_composed_payload`,
/// `sce-build/src/forge/static_lowering.rs`).
#[test]
fn the_payload_is_held_in_the_machine_types_each_record_declared_once() {
    let code = reader_machine();
    holds(
        &code,
        &[
            "type BatchReaderBatchSentPayload struct {\n\titems []BatchReaderItemRecord\n\tgrid \
             [][]uint8\n\tlevels []BatchReaderSignalLevelEnum\n\tmaybe []optional.Of[uint16]\n\t\
             spare optional.Of[[]uint8]\n\tnote string\n\tlate []uint8\n}",
            "type BatchReaderItemRecord struct {\n\tid uint32\n\tamount int64\n\tlabel string\n\t\
             level BatchReaderSignalLevelEnum\n\tdue optional.Of[int64]\n\tblob []byte\n\ttags \
             []string\n}",
            "func NewBatchReaderItemRecord(id uint32, amount int64, label string, level \
             BatchReaderSignalLevelEnum, due optional.Of[int64], blob []byte, tags []string) \
             BatchReaderItemRecord {\n\treturn BatchReaderItemRecord{id: id, amount: amount, \
             label: label, level: level, due: due, blob: blob, tags: tags}\n}",
        ],
    );
    assert_eq!(
        code.matches("type BatchReaderItemRecord struct").count(),
        1,
        "a record is declared once:\n{code}"
    );
}

/// Pinned by `GoComposed::read_list`, `GoComposed::read_record`,
/// `GoComposed::read_text`, `GoComposed::read_optional` and
/// `GoComposed::read_scalar` (`sce-build/src/forge/composed_lowering/go.rs`), the
/// lift's statements `go_composed_payload` writes
/// (`sce-build/src/forge/generator.rs`), and `composed_defaults_wire`
/// (`sce-build/src/forge/schema_revision.rs`): each field read by one runtime
/// call per constructor into the payload, a text held to its bound at the top
/// level too, a list the second revision added read as empty.
#[test]
fn the_lift_reads_each_field_by_its_type_at_every_depth() {
    let code = reader_machine();
    holds(
        &code,
        &[
            r#"fields, err := sce.LiftPayloadWithDefaults(meta.Data, "{\"late\":[]}")"#,
            "\t\t\tsceValue, err := fields.Field(\"items\")\n\t\t\tif err != nil {\n\t\t\t\treturn \
             err\n\t\t\t}\n",
            "if payload.items, err = sce.ReadList(sceValue, 3, func(sceValue sce.PayloadValue) \
             (BatchReaderItemRecord, error) { return func() (BatchReaderItemRecord, error) { \
             sceRecord, err := sceValue.Record(\"\"); ",
            "if sceOut.amount, err = sce.ValueSigned64(sceValue); err != nil",
            "if sceOut.label, err = sceValue.Text(8); err != nil",
            "if sceOut.due, err = optional.Read(sceValue, func(sceValue sce.PayloadValue) (int64, \
             error) { return sce.ValueSigned64(sceValue) }); err != nil",
            "if sceOut.tags, err = sce.ReadList(sceValue, 4, func(sceValue sce.PayloadValue) \
             (string, error) { return sceValue.Text(8) }); err != nil",
            "if payload.grid, err = sce.ReadList(sceValue, 2, func(sceValue sce.PayloadValue) \
             ([]uint8, error) { return sce.ReadList(sceValue, 2, func(sceValue sce.PayloadValue) \
             (uint8, error) { return sce.ValueUnsigned[uint8](sceValue) }) }); err != nil {",
            "if payload.maybe, err = sce.ReadList(sceValue, 2, func(sceValue sce.PayloadValue) \
             (optional.Of[uint16], error) { return optional.Read(sceValue, func(sceValue \
             sce.PayloadValue) (uint16, error) { return sce.ValueUnsigned[uint16](sceValue) }) \
             }); err != nil {",
            "if payload.spare, err = optional.Read(sceValue, func(sceValue sce.PayloadValue) \
             ([]uint8, error) { return sce.ReadList(sceValue, 2, func(sceValue sce.PayloadValue) \
             (uint8, error) { return sce.ValueUnsigned[uint8](sceValue) }) }); err != nil {",
            "if payload.note, err = sceValue.Text(4); err != nil {",
        ],
    );
}

/// Pinned by `go_raise_checks` and `go_bound_check`, `GoComposed::write_list`,
/// `GoComposed::write_record` and `GoComposed::write_optional`
/// (`sce-build/src/forge/composed_lowering/go.rs`), and by the prelude and the
/// answer `go_composed_payload` gives the inject seam
/// (`sce-build/src/forge/generator.rs`): the seam holds what a host hands it to
/// every bound its type declares, at any depth, before it builds anything
/// (docs/adr/0086, section 5.4), answers the refusal, and writes the payload it
/// builds as the JSON a lift reads back.
#[test]
fn the_inject_seam_checks_what_it_is_handed_and_writes_it_as_its_json() {
    let code = reader_machine();
    holds(
        &code,
        &[
            "func BatchReaderRaiseBatchSent(e *sce.Engine[BatchReaderState, BatchReaderEvent], \
             items []BatchReaderItemRecord, grid [][]uint8, levels []BatchReaderSignalLevelEnum, \
             maybe []optional.Of[uint16], spare optional.Of[[]uint8], note string, late []uint8) \
             error {\n",
            "\tif err := sce.CheckList(\"items\", items, 3, func(scePath0 string, sceValue0 \
             BatchReaderItemRecord) error { return func() error { if err := \
             sce.CheckText(scePath0 + \"/label\", sceValue0.label, 8); err != nil { return err }; \
             if err := sce.CheckBytes(scePath0 + \"/blob\", sceValue0.blob, 4); err != nil { \
             return err }; if err := sce.CheckList(scePath0 + \"/tags\", sceValue0.tags, 4, \
             func(scePath1 string, sceValue1 string) error { return sce.CheckText(scePath1, \
             sceValue1, 8) }); err != nil { return err }; return nil }() }); err != nil {\n\t\t\
             return err\n\t}\n",
            "\tif err := sce.CheckList(\"grid\", grid, 2, func(scePath0 string, sceValue0 \
             []uint8) error { return sce.CheckList(scePath0, sceValue0, 2, func(string, uint8) \
             error { return nil }) }); err != nil {\n",
            "\tif err := optional.Check(spare, func(sceValue0 []uint8) error { return \
             sce.CheckList(\"spare\", sceValue0, 2, func(string, uint8) error { return nil }) }); \
             err != nil {\n",
            "\tif err := sce.CheckText(\"note\", note, 4); err != nil {\n\t\treturn err\n\t}\n",
            "\tscePayload := BatchReaderBatchSentPayload{items: items, grid: grid, levels: levels, \
             maybe: maybe, spare: spare, note: note, late: late}\n",
            "\t\t\tTypedPayload: scePayload,\n",
            "\t\t\tData: string(sce.RecordJSON(sce.Pair(\"items\", func() sce.RawJSON { sceItems \
             := make([]sce.RawJSON, 0, len(scePayload.items)); for _, sceElement := range \
             scePayload.items { sceItems = append(sceItems, sce.RecordJSON(sce.Pair(\"id\", \
             sce.ValueJSON(sceElement.id)), sce.Pair(\"amount\", \
             sce.ValueJSON(sce.Int64PayloadText(sceElement.amount))), ",
            "sce.Pair(\"spare\", func() sce.RawJSON { if sceElement, sceIs := \
             (scePayload.spare).Get(); sceIs { return func() sce.RawJSON { sceItems := \
             make([]sce.RawJSON, 0, len(sceElement)); ",
            "sce.Pair(\"note\", sce.ValueJSON(scePayload.note))",
            "\t})\n\treturn nil\n}\n",
        ],
    );
}

/// Pinned by the loop over a list a place holds (`walked_place`,
/// `sce-build/src/forge/static_lowering.rs`), `GoTarget::foreach_loop`, and
/// the runtime's checked read of a list at an index, which takes a list of any
/// element.
#[test]
fn the_reads_lower_at_any_depth() {
    let code = reader_machine();
    holds(
        &code,
        &[
            "for sceIndex, row := range append(p.pendingBatchSentPayload.items[:0:0], \
             p.pendingBatchSentPayload.items...)",
            "for _, word := range append(row.tags[:0:0], row.tags...)",
            "for _, cells := range append(p.pendingBatchSentPayload.grid[:0:0], \
             p.pendingBatchSentPayload.grid...)",
            "for _, n := range append(cells[:0:0], cells...)",
            "p.pendingBatchSentPayload.items[:], ",
            "scealgorithm.At(&sceFailure, p.pendingBatchSentPayload.maybe[:], ",
        ],
    );
}

/// Pinned by `whole_list_copy` (`sce-build/src/forge/static_lowering.rs`),
/// `copier` and `GoComposed::copy_whole` (`sce-build/src/forge/composed_lowering.rs`,
/// `composed_lowering/go.rs`), and by `ComposedLowering::of`'s
/// `copies`: a list a payload holds is taken whole into a list variable of a
/// larger bound, one Go type, as a slice of its own, which the variable is
/// appended to and emptied in place without reaching the payload's.
#[test]
fn a_list_is_taken_whole_into_a_slice_of_its_own() {
    let code = fixture_machine("static_composed_payload.scxml");
    holds(
        &code,
        &["p.vLate = append(p.pendingBatchSentPayload.late[:0:0], \
           p.pendingBatchSentPayload.late...)"],
    );
}

/// Pinned by `composed_wire_value` (`sce-build/src/forge/static_lowering.rs`),
/// `GoTarget::composed_send_spelling` and the `GoCarrier::Wire` arms of
/// `GoComposed`'s writers (`sce-build/src/forge/composed_lowering/go.rs`): each
/// composed field of the forwarded delivery is written by walking its type
/// into the value a send holds its params in, which the runtime's
/// `ScriptValueToJSON` writes as the event's JSON.
#[test]
fn a_send_writes_each_composed_field_by_its_name() {
    let code = fixture_machine("static_composed_send.scxml");
    holds(
        &code,
        &[
            "sceValue := func() []any { sceItems := make([]any, 0, \
             len(p.pendingBatchSentPayload.items)); for _, sceElement := range \
             p.pendingBatchSentPayload.items { sceItems = append(sceItems, map[string]any{\"id\": \
             int64(sceElement.id), \"amount\": sce.Int64PayloadText(sceElement.amount), \"label\": \
             sceElement.label, \"level\": sceElement.level.String(), ",
            "sceValue := func() []any { sceItems := make([]any, 0, \
             len(p.pendingBatchSentPayload.levels)); for _, sceElement := range \
             p.pendingBatchSentPayload.levels { sceItems = append(sceItems, \
             sceElement.String()) }; return sceItems }()",
            "sceValue := func() any { if sceElement, sceIs := \
             (p.pendingBatchSentPayload.spare).Get(); sceIs { return func() []any { ",
            "sceValue := func() []any { sceItems := make([]any, 0, len(p.vLate)); for _, \
             sceElement := range p.vLate { sceItems = append(sceItems, int64(sceElement)) }; \
             return sceItems }()",
        ],
    );
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
/// Go takes the read, the whole copy and the send of both fixtures.
#[test]
fn go_takes_the_read_the_copy_and_the_send() {
    for (name, document) in [
        ("batch_reader.scxml", READER.to_string()),
        (
            "static_composed_payload.scxml",
            std::fs::read_to_string(fixtures().join("static_composed_payload.scxml"))
                .expect("fixture"),
        ),
        (
            "static_composed_send.scxml",
            std::fs::read_to_string(fixtures().join("static_composed_send.scxml"))
                .expect("fixture"),
        ),
    ] {
        let (ok, out) = run(
            &["check", "-l", "go"],
            &fixtures(),
            &SIBLINGS,
            name,
            &document,
        );
        assert!(ok, "Go lowers {name}:\n{out}");
    }
}

/// Pinned by the gate's `<send>` arm (`first_unlowered_composed_use`,
/// `sce-build/src/forge/event_schema_check.rs`): a composed field of the
/// schema's first revision is carried by every delivery (docs/adr/0086,
/// section 10.1), so a Go send that gives one no value is refused by name,
/// where a list a later revision adds may be left out (section 10.2).
#[test]
fn a_send_that_gives_a_first_revision_field_no_value_is_refused_on_go() {
    let text =
        std::fs::read_to_string(fixtures().join("static_composed_send.scxml")).expect("fixture");
    let without_items = text.replace(
        "        <param name=\"items\" expr=\"_event.data.items\"/>\n",
        "",
    );
    assert_ne!(without_items, text, "the fixture gives `items` by its name");
    let (ok, out) = run(
        &["check", "-l", "go"],
        &fixtures(),
        &SIBLINGS,
        "static_composed_send.scxml",
        &without_items,
    );
    assert!(!ok, "a send without `items` lacks its value:\n{out}");
    assert!(out.contains("generate/unsupported-feature"), "{out}");
    assert!(out.contains("the field `items`"), "{out}");
    assert!(
        out.contains("a statechart's <send> of its event that gives the field no value"),
        "{out}"
    );
    let without_late = text.replace("        <param name=\"late\" expr=\"late\"/>\n", "");
    assert_ne!(without_late, text, "the fixture gives `late` by its name");
    let (ok, out) = run(
        &["check", "-l", "go"],
        &fixtures(),
        &SIBLINGS,
        "static_composed_send.scxml",
        &without_late,
    );
    assert!(ok, "`late` is a list the second revision added:\n{out}");
}

/// Pinned by the fit the judge asks of a whole copy (`Judge`,
/// `sce-build/src/forge/static_datamodel.rs`) before any backend lowers it: a
/// list that may hold more than the variable it is taken into is refused on Go
/// where the document is checked, so the copy Go writes as a slice of the same
/// type never puts a list past its bound into a variable.
#[test]
fn a_copy_that_may_pass_the_variables_bound_is_refused_on_go() {
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
        &["check", "-l", "go"],
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

/// A reader of `shapes.sent` that tests the tag of the payload's `one`,
/// importing the tag's enum, the event and the records its cases hold.
const SHAPES_READER: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="shapes_reader">
  <sce:import kind="enum" src="shape_kind.scxml" as="Kind"/>
  <sce:import kind="event-schema" src="event_shapes_minimal.scxml" as="Shapes"/>
  <sce:import kind="event-schema" src="shape_radius_minimal.scxml" as="Radius"/>
  <sce:import kind="event-schema" src="shape_rect_minimal.scxml" as="Rect"/>
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
    "shape_radius_minimal.scxml",
    "shape_rect_minimal.scxml",
    "union_shape_minimal.scxml",
    "event_shapes_minimal.scxml",
];

/// Pinned by the Go row of the one table of what each backend lowers
/// (`ComposedLowering::of`, `sce-build/src/forge/event_schema_check.rs`): Go
/// holds a union (docs/adr/0092, section 7), so the gate's union arm
/// (`first_unlowered_composed_use`) lets a read of a payload that holds one
/// through, and the machine is generated
/// (`a_go_machine_reads_a_union_payload_by_its_tag.rs` holds what it is).
#[test]
fn a_payload_that_holds_a_union_is_read_on_go() {
    let unions = repo_root().join("sce-build/tests/fixtures/union");
    let (ok, out) = run(
        &["check", "-l", "go"],
        &unions,
        &SHAPES,
        "shapes_reader.scxml",
        SHAPES_READER,
    );
    assert!(ok, "Go reads a payload that holds a union:\n{out}");
    assert!(
        !out.contains("generate/unsupported-feature"),
        "nothing of the reader is refused on Go:\n{out}"
    );
}
