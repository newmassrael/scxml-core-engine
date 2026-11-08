// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A C++ `sce-static` machine reads the payload of an event whose schema holds
//! a composition, at any depth (docs/adr/0086, section 6), and sends a
//! composed value by its name (section 5.2), as the Rust, Kotlin, Go and
//! Python machines do: its payload struct holds each field in the C++ row of
//! section 7.1, each record it names is declared once as plain data a host
//! builds by position, the lift reads each field by one lambda per
//! constructor over the runtime's `EventPayloadValue` (section 4), the inject
//! seam holds what a host hands it to every bound its type declares before it
//! writes it as its JSON (sections 3.1 and 5.4), and the statechart's reads,
//! copies and sends lower to the machine's own types.
//!
//! # What is held
//!
//! * The payload struct, the record a list of it holds, and the enum a list
//!   and a record hold, each in the machine's own type.
//! * The lift: a list of records that hold a list, a list of lists, an
//!   optional element, a list that may be absent, a top-level text held to its
//!   bound, and the defaults of the list the second revision added.
//! * The inject seam: the checks it runs over what it is handed, the refusal
//!   it answers, and the payload it builds, written as the JSON a lift reads
//!   back.
//! * The reads: `<foreach>` over a list the payload holds and over a list an
//!   enclosing loop's item holds.
//! * A list taken whole into a variable, copied by value.
//! * A `<send>` that names each composed field's place, each written as the
//!   `ScriptValue` a send holds its params in, and one that gives a field of
//!   the first revision no value, refused.
//! * A payload that holds a union stays refused by name on C++, which holds
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

/// The head of every reader the C++ machine writes: a lambda of the value it
/// reads and the place it fills (`CppComposed`,
/// `sce-build/src/forge/composed_lowering/cpp.rs`).
const R: &str = "[&](const ::SCE::Common::EventPayloadValue &sceValue, auto &sceInto) -> \
                 ::SCE::Common::EventPayloadRefusal";

/// The C++ machine of `document`, written as `name` beside the batch schemas:
/// every file it is generated as, the header and the inline part, one after
/// the other.
fn cpp_machine(name: &str, document: &str) -> String {
    let dir = tempdir().expect("tempdir");
    for sibling in SIBLINGS {
        std::fs::copy(fixtures().join(sibling), dir.path().join(sibling)).expect("copy a sibling");
    }
    let path = dir.path().join(name);
    std::fs::write(&path, document).expect("write the document");
    let language = sce_build::generator::Language::Cpp;
    let output = compile_scxml_lang_typed(
        path.to_str().expect("a utf-8 path"),
        &find_template_dir_for(language),
        language,
    )
    .unwrap_or_else(|e| panic!("C++ lowers {name}: {e:?}"));
    let names: Vec<String> = output.files.iter().map(|(name, _)| name.clone()).collect();
    assert!(
        names.iter().any(|name| name.ends_with("_sm.h")),
        "the machine's header is generated among {names:?}"
    );
    output
        .files
        .into_iter()
        .map(|(_, code)| code)
        .collect::<Vec<_>>()
        .join("\n")
}

/// The C++ machine of [`READER`].
fn reader_machine() -> String {
    cpp_machine("batch_reader.scxml", READER)
}

/// The C++ machine of the fixture `name`.
fn fixture_machine(name: &str) -> String {
    let text = std::fs::read_to_string(fixtures().join(name)).expect("fixture");
    cpp_machine(name, &text)
}

/// `code` holds each of `lines`.
fn holds(code: &str, lines: &[&str]) {
    for line in lines {
        assert!(code.contains(line), "expected `{line}` in:\n{code}");
    }
}

/// Pinned by `CppComposed::record_type`, `CppComposed::list_type`,
/// `CppComposed::optional_type`, `CppComposed::text_type`,
/// `CppComposed::bytes_type` and `CppComposed::record_def`
/// (`sce-build/src/forge/composed_lowering/cpp.rs`), by `cpp_composed_payload`
/// (`sce-build/src/forge/generator.rs`), and by the declaration of each record
/// a payload names (`declare_composed_payload`,
/// `sce-build/src/forge/static_lowering.rs`).
#[test]
fn the_payload_is_held_in_the_machine_types_each_record_declared_once() {
    let code = reader_machine();
    holds(
        &code,
        &[
            "struct BatchReaderBatchSentPayload {\n    std::vector<BatchReaderItemRecord> items;\n    \
             std::vector<std::vector<uint8_t>> grid;\n    std::vector<BatchReaderSignalLevelEnum> \
             levels;\n    std::vector<std::optional<uint16_t>> maybe;\n    \
             std::optional<std::vector<uint8_t>> spare;\n    std::string note;\n    \
             std::vector<uint8_t> late;\n};",
            "struct BatchReaderItemRecord {\n    uint32_t id;\n    int64_t amount;\n    std::string \
             label;\n    BatchReaderSignalLevelEnum level;\n    std::optional<int64_t> due;\n    \
             std::vector<uint8_t> blob;\n    std::vector<std::string> tags;\n};",
        ],
    );
    assert_eq!(
        code.matches("struct BatchReaderItemRecord {").count(),
        1,
        "a record is declared once:\n{code}"
    );
}

/// Pinned by `CppComposed::read_list`, `CppComposed::read_record`,
/// `CppComposed::read_text`, `CppComposed::read_optional`,
/// `CppComposed::read_enum` and `CppComposed::read_scalar`
/// (`sce-build/src/forge/composed_lowering/cpp.rs`), the lift's statements
/// `cpp_composed_payload` writes (`sce-build/src/forge/generator.rs`), and
/// `composed_defaults_wire` (`sce-build/src/forge/schema_revision.rs`): each
/// field read by one lambda per constructor into the payload, a text held to
/// its bound at the top level too, a list the second revision added read as
/// empty.
#[test]
fn the_lift_reads_each_field_by_its_type_at_every_depth() {
    let code = reader_machine();
    let whole = format!("{R} {{ return sceValue.readInteger(sceInto); }}");
    holds(
        &code,
        &[
            r#"::std::string refusal = ::SCE::Common::EventPayloadFields::decode(meta.data, "{\"late\":[]}", fields);"#,
            &format!(
                "            {{\n                ::SCE::Common::EventPayloadValue sceValue;\n                \
                 refusal = fields.value(\"items\", sceValue);\n                if \
                 (!refusal.empty()) {{\n                    return refusal;\n                }}\n                \
                 refusal = ({R} {{ return sceValue.readList(3u, sceInto, {R} {{ return \
                 sceValue.readRecord(nullptr, "
            ),
            &format!(
                "sceRefused = ({R} {{ return sceValue.readWide(sceInto); }})(sceField, \
                 sceInto.amount);"
            ),
            &format!(
                "sceRefused = ({R} {{ return sceValue.readText(8u, sceInto); }})(sceField, \
                 sceInto.label);"
            ),
            &format!(
                "sceRefused = ({R} {{ return sceValue.readOptional(sceInto, {R} {{ return \
                 sceValue.readWide(sceInto); }}); }})(sceField, sceInto.due);"
            ),
            &format!(
                "sceRefused = ({R} {{ return sceValue.readList(4u, sceInto, {R} {{ return \
                 sceValue.readText(8u, sceInto); }}); }})(sceField, sceInto.tags);"
            ),
            "if (sceName == \"low\") { sceInto = BatchReaderSignalLevelEnum::Low; return {}; } ",
            "return sceValue.unknownVariant(sceName, \"SignalLevel\"); }",
            &format!(
                "refusal = ({R} {{ return sceValue.readList(2u, sceInto, {R} {{ return \
                 sceValue.readList(2u, sceInto, {whole}); }}); }})(sceValue, payload.grid);"
            ),
            &format!(
                "refusal = ({R} {{ return sceValue.readList(2u, sceInto, {R} {{ return \
                 sceValue.readOptional(sceInto, {whole}); }}); }})(sceValue, payload.maybe);"
            ),
            &format!(
                "refusal = ({R} {{ return sceValue.readOptional(sceInto, {R} {{ return \
                 sceValue.readList(2u, sceInto, {whole}); }}); }})(sceValue, payload.spare);"
            ),
            &format!(
                "refusal = ({R} {{ return sceValue.readText(4u, sceInto); }})(sceValue, \
                 payload.note);"
            ),
        ],
    );
}

/// Pinned by `cpp_raise_checks` and `cpp_bound_check`, `CppComposed::write_list`,
/// `CppComposed::write_record` and `CppComposed::write_optional`
/// (`sce-build/src/forge/composed_lowering/cpp.rs`), and by the prelude and the
/// answer `cpp_composed_payload` gives the inject seam
/// (`sce-build/src/forge/generator.rs`): the seam holds what a host hands it to
/// every bound its type declares, at any depth, before it builds anything
/// (docs/adr/0086, section 5.4), answers the refusal, and writes the payload it
/// builds as the JSON a lift reads back.
#[test]
fn the_inject_seam_checks_what_it_is_handed_and_writes_it_as_its_json() {
    let code = reader_machine();
    let b = "::SCE::Common::EventPayloadBounds";
    let nothing = "[](const ::std::string &, const auto &) -> ::std::string { return {}; }";
    let j = "::SCE::Common::EventPayloadFields::json";
    holds(
        &code,
        &[
            "    [[nodiscard]] ::std::string raiseBatchSent(std::vector<BatchReaderItemRecord> items, \
             std::vector<std::vector<uint8_t>> grid, std::vector<BatchReaderSignalLevelEnum> \
             levels, std::vector<std::optional<uint16_t>> maybe, \
             std::optional<std::vector<uint8_t>> spare, std::string note, std::vector<uint8_t> \
             late) {\n        ::std::string sceRefused;\n",
            &format!(
                "        sceRefused = {b}::list(::std::string(\"items\"), items, 3u, [&](const \
                 ::std::string &scePath0, const auto &sceValue0) -> ::std::string {{ return [&]() \
                 -> ::std::string {{ ::std::string sceRefused; sceRefused = {b}::text(scePath0 + \
                 \"/label\", sceValue0.label, 8u); if (!sceRefused.empty()) {{ return sceRefused; \
                 }} sceRefused = {b}::bytes(scePath0 + \"/blob\", sceValue0.blob, 4u); if \
                 (!sceRefused.empty()) {{ return sceRefused; }} sceRefused = {b}::list(scePath0 + \
                 \"/tags\", sceValue0.tags, 4u, [&](const ::std::string &scePath1, const auto \
                 &sceValue1) -> ::std::string {{ return {b}::text(scePath1, sceValue1, 8u); }}); \
                 if (!sceRefused.empty()) {{ return sceRefused; }} return sceRefused; }}(); }});\n        \
                 if (!sceRefused.empty()) {{\n            return sceRefused;\n        }}\n"
            ),
            &format!(
                "        sceRefused = {b}::list(::std::string(\"grid\"), grid, 2u, [&](const \
                 ::std::string &scePath0, const auto &sceValue0) -> ::std::string {{ return \
                 {b}::list(scePath0, sceValue0, 2u, {nothing}); }});\n"
            ),
            &format!("        sceRefused = {b}::list(::std::string(\"levels\"), levels, 2u, {nothing});\n"),
            &format!("        sceRefused = {b}::list(::std::string(\"maybe\"), maybe, 2u, {nothing});\n"),
            &format!(
                "        sceRefused = {b}::optional(::std::string(\"spare\"), spare, [&](const \
                 ::std::string &scePath0, const auto &sceValue0) -> ::std::string {{ return \
                 {b}::list(scePath0, sceValue0, 2u, {nothing}); }});\n"
            ),
            &format!(
                "        sceRefused = {b}::text(::std::string(\"note\"), note, 4u);\n        if \
                 (!sceRefused.empty()) {{\n            return sceRefused;\n        }}\n"
            ),
            "        BatchReaderBatchSentPayload scePayload{/*items=*/::std::move(items), \
             /*grid=*/::std::move(grid), /*levels=*/::std::move(levels), \
             /*maybe=*/::std::move(maybe), /*spare=*/::std::move(spare), \
             /*note=*/::std::move(note), /*late=*/::std::move(late)};\n",
            "        m.typedPayload = scePayload;\n",
            &format!(
                "        m.data = ::SCE::Common::EventPayloadFields::record({{{{\"items\", \
                 ::SCE::Common::EventPayloadFields::list(scePayload.items, [&](const auto \
                 &sceElement) -> ::std::string {{ return \
                 ::SCE::Common::EventPayloadFields::record({{{{\"id\", {j}(sceElement.id)}}, \
                 {{\"amount\", {j}(::std::to_string(sceElement.amount))}}, "
            ),
            &format!("{{\"note\", {j}(scePayload.note)}}"),
            "        this->raiseExternal(m);\n        return {};\n    }\n",
        ],
    );
}

/// Pinned by the loop over a list a place holds (`walked_place`,
/// `sce-build/src/forge/static_lowering.rs`) and `CppTarget::foreach_loop`: a
/// list the payload holds and a list an enclosing loop's item holds are each
/// walked as the copy the loop begins with.
#[test]
fn the_reads_lower_at_any_depth() {
    let code = reader_machine();
    holds(
        &code,
        &[
            ": sceIndexed(pendingBatchSentPayload_.items))",
            "const auto &word : sceCopy(row.tags))",
            "const auto &cells : sceCopy(pendingBatchSentPayload_.grid))",
            "const auto &n : sceCopy(cells))",
        ],
    );
}

/// Pinned by `whole_list_copy` (`sce-build/src/forge/static_lowering.rs`),
/// `copier` and `CppComposed::copy_whole` (`sce-build/src/forge/composed_lowering.rs`,
/// `composed_lowering/cpp.rs`), and by `ComposedLowering::of`'s
/// `copies`: a list a payload holds is taken whole into a list variable of a
/// larger bound, one C++ type, by the assignment, which copies the vector by
/// value, so the variable is appended to and emptied in place without reaching
/// the payload's.
#[test]
fn a_list_is_taken_whole_into_a_variable_as_a_copy_by_value() {
    let code = fixture_machine("static_composed_payload.scxml");
    holds(&code, &["v_late = pendingBatchSentPayload_.late;"]);
}

/// Pinned by `composed_wire_value` (`sce-build/src/forge/static_lowering.rs`),
/// `CppTarget::composed_send_spelling` and the `CppCarrier::Wire` arms of
/// `CppComposed`'s writers (`sce-build/src/forge/composed_lowering/cpp.rs`): each
/// composed field of the forwarded delivery is written by walking its type
/// into the `ScriptValue` a send holds its params in, which
/// `EventDataHelper::buildEventDataJson` writes as the event's JSON.
#[test]
fn a_send_writes_each_composed_field_by_its_name() {
    let code = fixture_machine("static_composed_send.scxml");
    let list = "[&](const auto &sceList) -> ScriptValue { auto sceItems = \
                ::std::make_shared<ScriptArray>(); for (const auto &sceElement : sceList) { \
                sceItems->elements.push_back(";
    let listed = "); } return ScriptValue(std::in_place_type<std::shared_ptr<ScriptArray>>, \
                  ::std::move(sceItems)); }";
    holds(
        &code,
        &[
            &format!(
                "ScriptValue paramValue = {list}ScriptValue(std::in_place_type<std::shared_ptr<\
                 ScriptObject>>, ::std::make_shared<ScriptObject>(ScriptObject{{{{\"id\", \
                 ScriptValue(std::in_place_type<int64_t>, static_cast<int64_t>(sceElement.id))}}, \
                 {{\"amount\", ScriptValue(std::in_place_type<std::string>, \
                 std::to_string(sceElement.amount))}}, "
            ),
            &format!(
                "ScriptValue paramValue = {list}ScriptValue(std::in_place_type<std::string>, \
                 ::std::string(sceLogName(sceElement))){listed}(pendingBatchSentPayload_.levels);"
            ),
            "ScriptValue paramValue = [&](const auto &sceOptional) -> ScriptValue { if \
             (!sceOptional.has_value()) { return ScriptValue(ScriptNull{}); } const auto \
             &sceElement = *sceOptional; return [&](const auto &sceList) -> ScriptValue { ",
            "}(pendingBatchSentPayload_.spare);",
            &format!(
                "ScriptValue paramValue = {list}ScriptValue(std::in_place_type<int64_t>, \
                 static_cast<int64_t>(sceElement)){listed}(v_late);"
            ),
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
/// C++ takes the read, the whole copy and the send of both fixtures.
#[test]
fn cpp_takes_the_read_the_copy_and_the_send() {
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
            &["check", "-l", "cpp"],
            &fixtures(),
            &SIBLINGS,
            name,
            &document,
        );
        assert!(ok, "C++ lowers {name}:\n{out}");
    }
}

/// Pinned by the gate's `<send>` arm (`first_unlowered_composed_use`,
/// `sce-build/src/forge/event_schema_check.rs`): a composed field of the
/// schema's first revision is carried by every delivery (docs/adr/0086,
/// section 10.1), so a C++ send that gives one no value is refused by name,
/// where a list a later revision adds may be left out (section 10.2).
#[test]
fn a_send_that_gives_a_first_revision_field_no_value_is_refused_on_cpp() {
    let text =
        std::fs::read_to_string(fixtures().join("static_composed_send.scxml")).expect("fixture");
    let without_items = text.replace(
        "        <param name=\"items\" expr=\"_event.data.items\"/>\n",
        "",
    );
    assert_ne!(without_items, text, "the fixture gives `items` by its name");
    let (ok, out) = run(
        &["check", "-l", "cpp"],
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
        &["check", "-l", "cpp"],
        &fixtures(),
        &SIBLINGS,
        "static_composed_send.scxml",
        &without_late,
    );
    assert!(ok, "`late` is a list the second revision added:\n{out}");
}

/// Pinned by the fit the judge asks of a whole copy (`Judge`,
/// `sce-build/src/forge/static_datamodel.rs`) before any backend lowers it: a
/// list that may hold more than the variable it is taken into is refused on
/// C++ where the document is checked, so the copy C++ writes as an assignment
/// of one type never puts a list past its bound into a variable.
#[test]
fn a_copy_that_may_pass_the_variables_bound_is_refused_on_cpp() {
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
        &["check", "-l", "cpp"],
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

/// Pinned by the union arm of the gate's read (`first_unlowered_composed_use`,
/// `sce-build/src/forge/event_schema_check.rs`) and by `union_reached`
/// (`sce-build/src/forge/composed_lowering.rs`): C++ reads a composed payload,
/// holds no union, and so refuses a read of a payload that holds one by its
/// own name, naming the field and the union.
#[test]
fn a_payload_that_holds_a_union_stays_refused_by_name_on_cpp() {
    let unions = repo_root().join("sce-build/tests/fixtures/union");
    let (ok, out) = run(
        &["check", "-l", "cpp"],
        &unions,
        &SHAPES,
        "shapes_reader.scxml",
        SHAPES_READER,
    );
    assert!(!ok, "C++ holds no union yet:\n{out}");
    for part in [
        "generate/unsupported-feature",
        "the field `one` holds `union:Shape`",
        "a statechart's read of `_event.data` has no cpp lowering of its union",
    ] {
        assert!(out.contains(part), "expected `{part}` in:\n{out}");
    }
}
