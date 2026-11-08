// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A Python `sce-static` machine reads the payload of an event whose schema
//! holds a composition, at any depth (docs/adr/0086, section 6), as the Rust
//! and Kotlin machines do: its payload class holds each field in the Python
//! row of section 7.1, each record it names is declared once, the lift reads
//! each field by the runtime's name of its type (section 4), the inject seam
//! writes each as its JSON (section 3.1), a value a host makes is held to every
//! bound and width its type declares where it is made (section 5.4), and the
//! statechart's reads lower to the machine's own types.
//!
//! # What is held
//!
//! * The payload class, the record a list of it holds, and the enum a list and
//!   a record hold, each in the machine's own type.
//! * The `__post_init__` of the payload class and of the record: one check of
//!   the runtime per field that holds a bound or a width.
//! * The lift: a list of records that hold a list, a list of lists, an optional
//!   element, a list that may be absent, a top-level text held to its bound,
//!   and the defaults of the list the second revision added.
//! * The inject seam: the payload it is handed, written as the JSON a lift
//!   reads back.
//! * The reads: `<foreach>` over a list the payload holds and over a list an
//!   enclosing loop's item holds, an index into a list of lists, a field of an
//!   element read through its index.
//! * A list a payload holds is taken whole into a list variable as a copy that
//!   shares no list with the payload.
//! * A payload that holds a union stays refused by name on Python, which holds
//!   no union yet (docs/adr/0092, section 7).

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use common::composed_lowering::refusing_or_skip;
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

/// The alias the generated module imports `sce_runtime.event_payload` by.
const P: &str = "sce_event_payload";

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

/// The Python machine of `document`, written as `name` beside the batch
/// schemas.
fn python_machine(name: &str, document: &str) -> String {
    let dir = tempdir().expect("tempdir");
    for sibling in SIBLINGS {
        std::fs::copy(fixtures().join(sibling), dir.path().join(sibling)).expect("copy a sibling");
    }
    let path = dir.path().join(name);
    std::fs::write(&path, document).expect("write the document");
    let language = sce_build::generator::Language::Python;
    let output = compile_scxml_lang_typed(
        path.to_str().expect("a utf-8 path"),
        &find_template_dir_for(language),
        language,
    )
    .unwrap_or_else(|e| panic!("Python lowers {name}: {e:?}"));
    let names: Vec<String> = output.files.iter().map(|(name, _)| name.clone()).collect();
    output
        .files
        .into_iter()
        .find(|(name, _)| name.ends_with("_sm.py"))
        .map(|(_, code)| code)
        .unwrap_or_else(|| panic!("the machine's Python is generated among {names:?}"))
}

/// The Python machine of [`READER`].
fn reader_machine() -> String {
    python_machine("batch_reader.scxml", READER)
}

/// `code` holds each of `lines`.
fn holds(code: &str, lines: &[&str]) {
    for line in lines {
        assert!(code.contains(line), "expected `{line}` in:\n{code}");
    }
}

/// Pinned by `PythonComposed::record_type`, `PythonComposed::list_type`,
/// `PythonComposed::optional_type`, `PythonComposed::text_type`,
/// `PythonComposed::bytes_type` and `PythonComposed::record_def`
/// (`sce-build/src/forge/composed_lowering/python.rs`), by `python_composed_payload`
/// (`sce-build/src/forge/generator.rs`), and by the declaration of each record a
/// payload names (`declare_composed_payload`,
/// `sce-build/src/forge/static_lowering.rs`).
#[test]
fn the_payload_is_held_in_the_machine_types_each_record_declared_once() {
    let code = reader_machine();
    holds(
        &code,
        &[
            "@dataclass(frozen=True)\nclass BatchReaderBatchSentPayload:\n",
            "    items: list[BatchReaderItemRecord]\n    grid: list[list[int]]\n    \
             levels: list[BatchReaderSignalLevelEnum]\n    maybe: list[Optional[int]]\n    \
             spare: Optional[list[int]]\n    note: str\n    late: list[int]\n",
            "@dataclass(frozen=True)\nclass BatchReaderItemRecord:\n",
            "    id: int\n    amount: int\n    label: str\n    level: BatchReaderSignalLevelEnum\n    \
             due: Optional[int]\n    blob: bytes\n    tags: list[str]\n",
            "import sce_runtime.event_payload as sce_event_payload\n",
        ],
    );
    assert_eq!(
        code.matches("class BatchReaderItemRecord:").count(),
        1,
        "a record is declared once:\n{code}"
    );
}

/// Pinned by `PythonComposed::field_checks` and `python_post_init`
/// (`sce-build/src/forge/composed_lowering/python.rs`), the `class_body`
/// `python_composed_payload` gives the payload class
/// (`sce-build/src/forge/generator.rs`) and the checks `declare_composed` hands
/// each record (`sce-build/src/forge/static_lowering.rs`): a value a host makes,
/// the payload it hands the inject seam and each record it builds for it, is
/// refused in its class's `__post_init__`, where it is made, at the first value
/// past a bound or a width its type declares at any depth (docs/adr/0086,
/// section 5.4). A record a list holds checks itself, so the list is held to
/// its count.
#[test]
fn a_value_a_host_makes_is_held_to_every_bound_where_it_is_made() {
    let code = reader_machine();
    let wide = format!("{P}.WideOf(-9223372036854775808, 9223372036854775807)");
    holds(
        &code,
        &[
            "    late: list[int]\n\n    def __post_init__(self) -> None:\n",
            &format!("        {P}.check_value(\"items\", {P}.ListOf(None, 3), self.items)\n"),
            &format!(
                "        {P}.check_value(\"grid\", {P}.ListOf({P}.ListOf({P}.Whole(0, 255), 2), \
                 2), self.grid)\n"
            ),
            &format!("        {P}.check_value(\"levels\", {P}.ListOf(None, 2), self.levels)\n"),
            &format!(
                "        {P}.check_value(\"maybe\", {P}.ListOf({P}.OptionalOf({P}.Whole(0, \
                 65535)), 2), self.maybe)\n"
            ),
            &format!(
                "        {P}.check_value(\"spare\", {P}.OptionalOf({P}.ListOf({P}.Whole(0, 255), \
                 2)), self.spare)\n"
            ),
            &format!("        {P}.check_value(\"note\", {P}.Text(4), self.note)\n"),
            &format!(
                "        {P}.check_value(\"late\", {P}.ListOf({P}.Whole(0, 255), 2), self.late)\n"
            ),
            "    tags: list[str]\n\n    def __post_init__(self) -> None:\n",
            &format!("        {P}.check_value(\"id\", {P}.Whole(0, 4294967295), self.id)\n"),
            &format!("        {P}.check_value(\"amount\", {wide}, self.amount)\n"),
            &format!("        {P}.check_value(\"label\", {P}.Text(8), self.label)\n"),
            &format!("        {P}.check_value(\"due\", {P}.OptionalOf({wide}), self.due)\n"),
            &format!("        {P}.check_value(\"blob\", {P}.Bytes(4), self.blob)\n"),
            &format!("        {P}.check_value(\"tags\", {P}.ListOf({P}.Text(8), 4), self.tags)\n"),
        ],
    );
}

/// Pinned by the fit the judge asks of a whole copy (`Judge`,
/// `sce-build/src/forge/static_datamodel.rs`) before any backend lowers it: a
/// list that may hold more than the variable it is taken into is refused on
/// Python where the document is checked, so the copy Python writes never puts a
/// list past its bound into a variable.
#[test]
fn a_copy_that_may_pass_the_variables_bound_is_refused_on_python() {
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
        &["check", "-l", "python"],
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

/// Pinned by `PythonComposed::read_list`, `PythonComposed::read_record`,
/// `PythonComposed::read_text`, `PythonComposed::read_optional`,
/// `PythonComposed::read_enum` and `PythonComposed::read_scalar`, and by
/// `composed_defaults_wire` (`sce-build/src/forge/schema_revision.rs`): each
/// field read by the runtime's name of its type, a text held to its bound at
/// the top level too, a list the second revision added read as empty.
#[test]
fn the_lift_reads_each_field_by_its_type_at_every_depth() {
    let code = reader_machine();
    let wide = format!("{P}.WideOf(-9223372036854775808, 9223372036854775807)");
    holds(
        &code,
        &[
            &format!(
                "metadata.data, ((\"items\", {P}.ListOf({P}.RecordOf([(\"id\", {P}.Whole(0, \
                 4294967295)), (\"amount\", {wide}), (\"label\", {P}.Text(8)), (\"level\", \
                 BatchReaderSignalLevelEnum), (\"due\", {P}.OptionalOf({wide})), (\"blob\", \
                 {P}.Bytes(4)), (\"tags\", {P}.ListOf({P}.Text(8), 4))], None, \
                 BatchReaderItemRecord), 3)), "
            ),
            &format!(
                "(\"grid\", {P}.ListOf({P}.ListOf({P}.Whole(0, 255), 2), 2)), (\"levels\", \
                 {P}.ListOf(BatchReaderSignalLevelEnum, 2)), (\"maybe\", \
                 {P}.ListOf({P}.OptionalOf({P}.Whole(0, 65535)), 2)), (\"spare\", \
                 {P}.OptionalOf({P}.ListOf({P}.Whole(0, 255), 2))), (\"note\", {P}.Text(4)), \
                 (\"late\", {P}.ListOf({P}.Whole(0, 255), 2)), ), \"{{\\\"late\\\":[]}}\"))"
            ),
        ],
    );
}

/// Pinned by `PythonComposed::write_list`, `PythonComposed::write_record`,
/// `PythonComposed::write_optional`, `PythonComposed::write_enum` and
/// `PythonComposed::write_scalar` under `PythonCarrier::Json`, and by the
/// prelude `python_composed_payload` gives the inject seam: the seam builds the
/// payload it is handed, which its class checks, and writes it as the JSON a
/// lift reads back.
#[test]
fn the_inject_seam_writes_each_field_as_its_json() {
    let code = reader_machine();
    holds(
        &code,
        &[
            "def raise_batch_sent(engine, items: list[BatchReaderItemRecord], grid: \
             list[list[int]], levels: list[BatchReaderSignalLevelEnum], maybe: \
             list[Optional[int]], spare: Optional[list[int]], note: str, late: list[int]) -> \
             None:\n",
            "    sce_payload = BatchReaderBatchSentPayload(items, grid, levels, maybe, spare, \
             note, late)\n    engine.send_event(\n",
            "            typed_payload=sce_payload,\n",
            &format!(
                "            data={P}.record_json({{\"items\": {P}.list_json(sce_payload.items, \
                 lambda sce_element: {P}.record_json({{\"id\": {P}.value_json(sce_element.id), \
                 \"amount\": {P}.wide_json(sce_element.amount), "
            ),
            &format!("\"level\": {P}.value_json((sce_element.level).sce_name), "),
            &format!(
                "\"due\": {P}.value_json({P}.write_optional(sce_element.due, lambda \
                 sce_element: {P}.wide_json(sce_element))), "
            ),
            &format!(
                "\"grid\": {P}.list_json(sce_payload.grid, lambda sce_element: \
                 {P}.list_json(sce_element, lambda sce_element: {P}.value_json(sce_element)))"
            ),
            &format!(
                "\"spare\": {P}.value_json({P}.write_optional(sce_payload.spare, lambda \
                 sce_element: {P}.list_json(sce_element, lambda sce_element: \
                 {P}.value_json(sce_element))))"
            ),
            &format!("\"note\": {P}.value_json(sce_payload.note)"),
        ],
    );
}

/// Pinned by the loop over a list a place holds (`walked_place`,
/// `sce-build/src/forge/static_lowering.rs`), `PythonTarget::foreach_loop`, and
/// the runtime's checked read of a list at an index, which takes a list of any
/// element.
#[test]
fn the_reads_lower_at_any_depth() {
    let code = reader_machine();
    holds(
        &code,
        &[
            "in enumerate(list(self._pending_batch_sent_payload.items)):",
            "for word in list(row.tags):",
            "for cells in list(self._pending_batch_sent_payload.grid):",
            "for n in list(cells):",
            "sce_algorithm.at(self._pending_batch_sent_payload.items, ",
            "sce_algorithm.at(sce_algorithm.at(self._pending_batch_sent_payload.grid, ",
            "sce_algorithm.at(self._pending_batch_sent_payload.maybe, ",
        ],
    );
}

/// Pinned by `whole_list_copy` (`sce-build/src/forge/static_lowering.rs`),
/// `copier` and `PythonComposed::copy_whole` (`sce-build/src/forge/composed_lowering.rs`,
/// `composed_lowering/python.rs`), and by `ComposedLowering::of`'s
/// `copies`: a list a payload holds is taken whole into a list variable of a
/// larger bound, one Python type, as a copy that shares no list with the
/// payload, since the variable is written into in place.
#[test]
fn a_list_is_taken_whole_into_a_variable_whose_type_it_fits() {
    let text =
        std::fs::read_to_string(fixtures().join("static_composed_payload.scxml")).expect("fixture");
    let code = python_machine("static_composed_payload.scxml", &text);
    holds(
        &code,
        &["self.v_late = sce_algorithm.whole_copy(self._pending_batch_sent_payload.late)"],
    );
    let (ok, out) = run(
        &["check", "-l", "python"],
        &fixtures(),
        &SIBLINGS,
        "static_composed_payload.scxml",
        &text,
    );
    assert!(ok, "Python lowers the fixture:\n{out}");
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
/// Python takes the read, and each backend whose row does not read a composed
/// payload, asked of the table, refuses it by its own name.
#[test]
fn python_takes_the_read_and_another_backend_refuses_it_by_name() {
    let (ok, out) = run(
        &["check", "-l", "python"],
        &fixtures(),
        &SIBLINGS,
        "batch_reader.scxml",
        READER,
    );
    assert!(ok, "Python reads a composed payload:\n{out}");
    for language in refusing_or_skip(
        |row| row.reads,
        "python_takes_the_read_and_another_backend_refuses_it_by_name",
    ) {
        let (ok, out) = run(
            &["check", "-l", language],
            &fixtures(),
            &SIBLINGS,
            "batch_reader.scxml",
            READER,
        );
        assert!(
            !ok,
            "{language} does not read a composed payload yet:\n{out}"
        );
        assert!(out.contains("generate/unsupported-feature"), "{out}");
        assert!(
            out.contains(&format!(
                "a statechart's read of `_event.data` has no {language} lowering of its list"
            )),
            "{out}"
        );
    }
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

/// Pinned by the union arm of the gate's read (`first_unlowered_composed_use`,
/// `sce-build/src/forge/event_schema_check.rs`) and by `union_reached`
/// (`sce-build/src/forge/composed_lowering.rs`): Python reads a composed
/// payload, holds no union, and so refuses a read of a payload that holds one
/// by its own name, naming the field and the union.
#[test]
fn a_payload_that_holds_a_union_stays_refused_by_name_on_python() {
    let unions = repo_root().join("sce-build/tests/fixtures/union");
    let (ok, out) = run(
        &["check", "-l", "python"],
        &unions,
        &SHAPES,
        "shapes_reader.scxml",
        SHAPES_READER,
    );
    assert!(!ok, "Python holds no union yet:\n{out}");
    for part in [
        "generate/unsupported-feature",
        "the field `one` holds `union:Shape`",
        "a statechart's read of `_event.data` has no python lowering of its union",
    ] {
        assert!(out.contains(part), "expected `{part}` in:\n{out}");
    }
}
