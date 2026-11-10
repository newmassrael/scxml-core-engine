// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A Swift `sce-static` machine reads the payload of an event whose schema
//! holds a composition, at any depth (docs/adr/0086, section 6), and sends a
//! composed value by its name (section 5.2), as the Rust, Kotlin, Go, Python
//! and C++ machines do: its payload struct holds each field in the Swift row
//! of section 7.1, each record it names is declared once with the initializer
//! a host builds one by, the lift reads each field by one runtime call per
//! constructor (section 4), the inject seam holds what a host hands it to
//! every bound its type declares before it writes it as its JSON (sections
//! 3.1 and 5.4), and the statechart's reads, copies and sends lower to the
//! machine's own types.
//!
//! Swift lowers `datamodel="sce-static"` in part (`lowers_static_datamodel`
//! is false for it), so the suites that ask the table which whole backends
//! lower a composed position do not ask it; this file asks it of each
//! position its row lowers, and of the union its row does not.
//!
//! # What is held
//!
//! * The payload struct, the record a list of it holds, and the enum a list
//!   and a record hold, each in the machine's own type, each member at its
//!   zero.
//! * The lift: a list of records that hold a list, a list of lists, an
//!   optional element, a list that may be absent, a top-level text held to its
//!   bound, and the defaults of the list the second revision added.
//! * The inject seam: the checks it runs over what it is handed, and the
//!   payload it builds, written as the JSON a lift reads back.
//! * The reads: `<foreach>` over a list the payload holds, an index into a
//!   list of records, of lists and of values that may be absent.
//! * A list taken whole into a variable, as the value it is.
//! * A `<send>` that names each composed field's place, each written as the
//!   `WireValue` a send holds its params in, and one that gives a field of the
//!   first revision no value, refused.
//! * A payload that holds a union stays refused by name on Swift, which holds
//!   no union yet (docs/adr/0092, section 7).

use std::path::{Path, PathBuf};
use std::process::Command;

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

/// `sce-codegen <args> <file>` over `document` written as `name` in a
/// directory that holds `siblings` of `from`, and whether it succeeded, with
/// what it said; `out`, when given, is where a `generate` writes.
fn run_in(
    args: &[&str],
    from: &Path,
    siblings: &[&str],
    name: &str,
    document: &str,
    out: Option<&Path>,
) -> (bool, String) {
    let dir = tempdir().expect("tempdir");
    for sibling in siblings {
        std::fs::copy(from.join(sibling), dir.path().join(sibling)).expect("copy a sibling");
    }
    std::fs::write(dir.path().join(name), document).expect("write the document");
    let mut command = Command::new(env!("CARGO_BIN_EXE_sce-codegen"));
    command
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(args)
        .arg(dir.path().join(name));
    if let Some(out) = out {
        command.arg("-o").arg(out);
    }
    let output = command.output().expect("invoke sce-codegen");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stderr).into_owned()
            + &String::from_utf8_lossy(&output.stdout),
    )
}

/// [`run_in`] with nothing written.
fn run(
    args: &[&str],
    from: &Path,
    siblings: &[&str],
    name: &str,
    document: &str,
) -> (bool, String) {
    run_in(args, from, siblings, name, document, None)
}

/// The Swift machine of `document`, written as `name` beside the batch schemas.
fn swift_machine(name: &str, document: &str) -> String {
    let out = tempdir().expect("tempdir");
    let (ok, said) = run_in(
        &["generate", "-l", "swift"],
        &fixtures(),
        &SIBLINGS,
        name,
        document,
        Some(out.path()),
    );
    assert!(ok, "Swift lowers {name}:\n{said}");
    let stem = name.trim_end_matches(".scxml");
    std::fs::read_to_string(out.path().join(format!("{stem}_sm.swift")))
        .unwrap_or_else(|_| panic!("{name} writes {stem}_sm.swift"))
}

/// The Swift machine of [`READER`].
fn reader_machine() -> String {
    swift_machine("batch_reader.scxml", READER)
}

/// The Swift machine of the fixture `name`.
fn fixture_machine(name: &str) -> String {
    let text = std::fs::read_to_string(fixtures().join(name)).expect("fixture");
    swift_machine(name, &text)
}

/// `code` holds each of `lines`.
fn holds(code: &str, lines: &[&str]) {
    for line in lines {
        assert!(code.contains(line), "expected `{line}` in:\n{code}");
    }
}

/// Pinned by `SwiftComposed::record_type`, `SwiftComposed::list_type`,
/// `SwiftComposed::optional_type`, `SwiftComposed::record_def` and
/// `SwiftComposed::zero_of` (`sce-build/src/forge/composed_lowering/swift.rs`),
/// by `swift_composed_payload` (`sce-build/src/forge/generator.rs`), and by the
/// declaration of each record a payload names (`declare_composed_payload`,
/// `sce-build/src/forge/static_lowering.rs`).
#[test]
fn the_payload_is_held_in_the_machine_types_each_record_declared_once() {
    let code = reader_machine();
    holds(
        &code,
        &[
            "struct BatchReaderBatchSentPayload {\n    var items: [BatchReaderItemRecord] = []\n    \
             var grid: [[UInt8]] = []\n    var levels: [BatchReaderSignalLevelEnum] = []\n    \
             var maybe: [UInt16?] = []\n    var spare: [UInt8]? = nil\n    var note: String = \
             \"\"\n    var late: [UInt8] = []\n}",
            "public struct BatchReaderItemRecord: Equatable, Sendable, SceDefault {\n    public \
             var id: UInt32 = 0\n    public var amount: Int64 = 0\n    public var label: String \
             = \"\"\n    public var level: BatchReaderSignalLevelEnum = \
             BatchReaderSignalLevelEnum.allCases[0]\n    public var due: Int64? = nil\n    public \
             var blob: [UInt8] = []\n    public var tags: [String] = []\n\n    public init() {}\n",
            "    public init(id: UInt32, amount: Int64, label: String, level: \
             BatchReaderSignalLevelEnum, due: Int64?, blob: [UInt8], tags: [String]) {\n",
        ],
    );
    assert_eq!(
        code.matches("public struct BatchReaderItemRecord:").count(),
        1,
        "a record is declared once:\n{code}"
    );
}

/// A reader of `batch.sent` that holds no variable: its guard alone reads the
/// payload, whose record the machine declares all the same.
const GUARD_ONLY: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="batch_guard">
  <sce:import kind="event-schema" src="event_batch.scxml" as="Batch"/>
  <sce:import kind="event-schema" src="event_batch_item.scxml" as="Item"/>
  <sce:import kind="enum" src="enum_signal_level.scxml" as="SignalLevel"/>
  <state id="idle">
    <transition event="batch.sent" cond="len(_event.data.items) &gt; 0" target="full"/>
  </state>
  <state id="full"/>
</scxml>
"#;

/// Pinned by the import of `state_machine.swift.jinja2`
/// (`tools/codegen/templates/swift/`): a record the machine declares is the
/// forge runtime's `SceDefault`, so a machine that holds no variable and only
/// reads a payload that names a record imports the forge runtime too.
#[test]
fn a_machine_of_no_variable_imports_the_runtime_its_records_name() {
    let code = swift_machine("batch_guard.scxml", GUARD_ONLY);
    holds(
        &code,
        &[
            "public struct BatchGuardItemRecord: Equatable, Sendable, SceDefault {\n",
            "\nimport SceForgeRuntime\n",
        ],
    );
}

/// Pinned by `SwiftComposed::read_list`, `SwiftComposed::read_record`,
/// `SwiftComposed::read_text`, `SwiftComposed::read_optional` and
/// `SwiftComposed::read_scalar` (`sce-build/src/forge/composed_lowering/swift.rs`),
/// the lift's statements `swift_composed_payload` writes
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
            r#"let fields = try PayloadFields.decode(metadata.data, defaults: "{\"late\":[]}")"#,
            "            payload.items = try ({ (sceValue: PayloadValue) throws -> \
             [BatchReaderItemRecord] in try sceValue.list(bound: 3, { (sceValue: PayloadValue) \
             throws -> BatchReaderItemRecord in try sceValue.record({ (sceFields: PayloadFields) \
             throws -> BatchReaderItemRecord in try BatchReaderItemRecord(id: ({ (sceValue: \
             PayloadValue) throws -> UInt32 in try sceValue.unsigned() \
             })(sceFields.field(\"id\")), ",
            "amount: ({ (sceValue: PayloadValue) throws -> Int64 in try sceValue.signed64() \
             })(sceFields.field(\"amount\")), ",
            "label: ({ (sceValue: PayloadValue) throws -> String in try sceValue.text(bound: 8) \
             })(sceFields.field(\"label\")), ",
            "level: ({ (sceValue: PayloadValue) throws -> BatchReaderSignalLevelEnum in let \
             sceName = try sceValue.name(); guard let sceVariant = \
             BatchReaderSignalLevelEnum(declaredName: sceName) else { throw \
             sceValue.unknownVariant(sceName, alias: \"SignalLevel\") }; return sceVariant \
             })(sceFields.field(\"level\")), ",
            "due: ({ (sceValue: PayloadValue) throws -> Int64? in try sceValue.optional({ \
             (sceValue: PayloadValue) throws -> Int64 in try sceValue.signed64() }) \
             })(sceFields.field(\"due\")), ",
            "tags: ({ (sceValue: PayloadValue) throws -> [String] in try sceValue.list(bound: 4, \
             { (sceValue: PayloadValue) throws -> String in try sceValue.text(bound: 8) }) \
             })(sceFields.field(\"tags\"))) }) }) })(fields.field(\"items\"))\n",
            "            payload.grid = try ({ (sceValue: PayloadValue) throws -> [[UInt8]] in try \
             sceValue.list(bound: 2, { (sceValue: PayloadValue) throws -> [UInt8] in try \
             sceValue.list(bound: 2, { (sceValue: PayloadValue) throws -> UInt8 in try \
             sceValue.unsigned() }) }) })(fields.field(\"grid\"))\n",
            "            payload.maybe = try ({ (sceValue: PayloadValue) throws -> [UInt16?] in try \
             sceValue.list(bound: 2, { (sceValue: PayloadValue) throws -> UInt16? in try \
             sceValue.optional({ (sceValue: PayloadValue) throws -> UInt16 in try \
             sceValue.unsigned() }) }) })(fields.field(\"maybe\"))\n",
            "            payload.spare = try ({ (sceValue: PayloadValue) throws -> [UInt8]? in try \
             sceValue.optional({ (sceValue: PayloadValue) throws -> [UInt8] in try \
             sceValue.list(bound: 2, { (sceValue: PayloadValue) throws -> UInt8 in try \
             sceValue.unsigned() }) }) })(fields.field(\"spare\"))\n",
            "            payload.note = try ({ (sceValue: PayloadValue) throws -> String in try \
             sceValue.text(bound: 4) })(fields.field(\"note\"))\n",
        ],
    );
}

/// Pinned by `swift_raise_checks` and `swift_bound_check`,
/// `SwiftComposed::write_list`, `SwiftComposed::write_record` and
/// `SwiftComposed::write_optional`
/// (`sce-build/src/forge/composed_lowering/swift.rs`), and by the prelude and
/// the `throws` `swift_composed_payload` gives the inject seam
/// (`sce-build/src/forge/generator.rs`): the seam holds what a host hands it to
/// every bound its type declares, at any depth, before it builds anything
/// (docs/adr/0086, section 5.4), throws the refusal, and writes the payload it
/// builds as the JSON a lift reads back.
#[test]
fn the_inject_seam_checks_what_it_is_handed_and_writes_it_as_its_json() {
    let code = reader_machine();
    holds(
        &code,
        &[
            "    public func raiseBatchSent(items: [BatchReaderItemRecord], grid: [[UInt8]], \
             levels: [BatchReaderSignalLevelEnum], maybe: [UInt16?], spare: [UInt8]?, note: \
             String, late: [UInt8]) throws {\n",
            "        try checkList(\"items\", items, bound: 3, { scePath0, sceValue0 in try \
             checkText(scePath0 + \"/label\", sceValue0.label, bound: 8); try \
             checkBytes(scePath0 + \"/blob\", sceValue0.blob, bound: 4); try \
             checkList(scePath0 + \"/tags\", sceValue0.tags, bound: 4, { scePath1, sceValue1 \
             in try checkText(scePath1, sceValue1, bound: 8) }) })\n",
            "        try checkList(\"grid\", grid, bound: 2, { scePath0, sceValue0 in try \
             checkList(scePath0, sceValue0, bound: 2, { _, _ in }) })\n",
            "        try checkList(\"levels\", levels, bound: 2, { _, _ in })\n",
            "        try checkOptional(\"spare\", spare, { scePath0, sceValue0 in try \
             checkList(scePath0, sceValue0, bound: 2, { _, _ in }) })\n",
            "        try checkText(\"note\", note, bound: 4)\n",
            "        let scePayload = BatchReaderBatchSentPayload(items: items, grid: grid, levels: \
             levels, maybe: maybe, spare: spare, note: note, late: late)\n",
            "                    typedPayload: scePayload\n",
            "                    data: recordJSON([PayloadPair.json(\"items\", rawJSON: \
             listJSON(scePayload.items, { (sceElement) -> String in \
             recordJSON([PayloadPair.json(\"id\", rawJSON: \
             WireValue.integer(Int64(sceElement.id)).json), PayloadPair.json(\"amount\", \
             rawJSON: WireValue.signed64(sceElement.amount).json), ",
            "PayloadPair.json(\"spare\", rawJSON: optionalJSON(scePayload.spare, write: { \
             (sceElement) -> String in listJSON(sceElement, { (sceElement) -> String in \
             WireValue.integer(Int64(sceElement)).json }) })), ",
            "PayloadPair.json(\"note\", rawJSON: WireValue.text(scePayload.note).json)",
        ],
    );
}

/// Pinned by the loop over a list a place holds (`walked_place`,
/// `sce-build/src/forge/static_lowering.rs`), `SwiftTarget::foreach_loop`, and
/// the checked read of an element (`element_fallback`,
/// `sce-build/src/forge/expr/swift.rs`): a record element stands at its zero
/// where the index is outside the list, and an element that may be absent at
/// its absence.
#[test]
fn the_reads_lower_at_any_depth() {
    let code = reader_machine();
    holds(
        &code,
        &[
            " in self.pendingBatchSentPayload.items.enumerated()",
            " in self.pendingBatchSentPayload.grid",
            "Checked.atDefault(sceFailure, self.pendingBatchSentPayload.items, ",
            "Checked.at(sceFailure, self.pendingBatchSentPayload.maybe, ",
            "fallback: nil)",
        ],
    );
}

/// Pinned by `element_fallback` (`sce-build/src/forge/expr/swift.rs`): an
/// element of a list of lists, read by its index, stands as the empty list
/// where the index is outside the list.
#[test]
fn an_element_of_a_list_of_lists_falls_back_to_the_empty_list() {
    let code = fixture_machine("static_composed_payload.scxml");
    holds(
        &code,
        &[
            "Checked.at(sceFailure, self.pendingBatchSentPayload.grid, ",
            "fallback: [])",
        ],
    );
}

/// Pinned by `whole_list_copy` (`sce-build/src/forge/static_lowering.rs`),
/// `copier` and `SwiftComposed::copy_whole`
/// (`sce-build/src/forge/composed_lowering.rs`, `composed_lowering/swift.rs`),
/// and by `ComposedLowering::of`'s `copies`: a list a payload holds is taken
/// whole into a list variable of a larger bound, one Swift type, as the value
/// it is, which the assignment copies.
#[test]
fn a_list_is_taken_whole_as_the_value_it_is() {
    let code = fixture_machine("static_composed_payload.scxml");
    holds(&code, &["self.vLate = self.pendingBatchSentPayload.late"]);
}

/// Pinned by `composed_wire_value` (`sce-build/src/forge/static_lowering.rs`),
/// `SwiftTarget::composed_send_spelling` and `SwiftComposed::send_param` with
/// the `SwiftCarrier::Wire` carrier
/// (`sce-build/src/forge/composed_lowering/swift.rs`): each composed field of
/// the forwarded delivery is written by walking its type, and held as the
/// `WireValue.composed` a send holds its params in.
#[test]
fn a_send_writes_each_composed_field_by_its_name() {
    let code = fixture_machine("static_composed_send.scxml");
    holds(
        &code,
        &[
            "let sceValue: WireValue = WireValue.composed(listJSON(self.pendingBatchSentPayload.\
             items, { (sceElement) -> String in recordJSON([PayloadPair.json(\"id\", rawJSON: \
             WireValue.integer(Int64(sceElement.id)).json), ",
            "let sceValue: WireValue = WireValue.composed(listJSON(self.pendingBatchSentPayload.\
             levels, { (sceElement) -> String in WireValue.text(sceElement.declaredName).json \
             }))",
            "let sceValue: WireValue = WireValue.composed(optionalJSON(self.pendingBatchSentPayload.\
             spare, write: { (sceElement) -> String in listJSON(sceElement, { (sceElement) -> \
             String in WireValue.integer(Int64(sceElement)).json }) }))",
            "let sceValue: WireValue = WireValue.composed(listJSON(self.vLate, { (sceElement) -> \
             String in WireValue.integer(Int64(sceElement)).json }))",
        ],
    );
}

/// Pinned by `ComposedLowering::of` (`sce-build/src/forge/event_schema_check.rs`)
/// and `SwiftTarget::composed_spelling`: Swift takes the read, the whole copy
/// and the send of both fixtures.
#[test]
fn swift_takes_the_read_the_copy_and_the_send() {
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
            &["check", "-l", "swift"],
            &fixtures(),
            &SIBLINGS,
            name,
            &document,
        );
        assert!(ok, "Swift lowers {name}:\n{out}");
    }
}

/// Pinned by the gate's `<send>` arm (`first_unlowered_composed_use`,
/// `sce-build/src/forge/event_schema_check.rs`): a composed field of the
/// schema's first revision is carried by every delivery (docs/adr/0086,
/// section 10.1), so a Swift send that gives one no value is refused by name,
/// where a list a later revision adds may be left out (section 10.2).
#[test]
fn a_send_that_gives_a_first_revision_field_no_value_is_refused_on_swift() {
    let text =
        std::fs::read_to_string(fixtures().join("static_composed_send.scxml")).expect("fixture");
    let without_items = text.replace(
        "        <param name=\"items\" expr=\"_event.data.items\"/>\n",
        "",
    );
    assert_ne!(without_items, text, "the fixture gives `items` by its name");
    let (ok, out) = run(
        &["check", "-l", "swift"],
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
        &["check", "-l", "swift"],
        &fixtures(),
        &SIBLINGS,
        "static_composed_send.scxml",
        &without_late,
    );
    assert!(ok, "`late` is a list the second revision added:\n{out}");
}

/// A field of a record that holds a composed value is a member of the struct
/// Swift holds it in and a label of its initializer, so a name Swift reserves is
/// refused by name rather than written as a member that does not compile. The
/// document is judged for the reserved words of the language it is built for
/// before the Swift gate reads it (`validation/reserved-code-identifier`, SCE
/// Accepted Subset §2.14), so that is the refusal a build for Swift answers; the
/// gate's own sentence of the same field (`SwiftTarget::unsupported`,
/// `sce-build/src/forge/static_lowering.rs`) backs it for a route that skips the
/// validation.
#[test]
fn a_composed_record_whose_field_swift_reserves_is_refused_by_name() {
    let item =
        std::fs::read_to_string(fixtures().join("event_batch_item.scxml")).expect("the item");
    // A word only Swift reserves, so the refusal names Swift and no other
    // backend: `default` is reserved in C++ as well, and is refused for that.
    let reserved = item.replace("<data id=\"label\"", "<data id=\"fileprivate\"");
    assert_ne!(reserved, item, "the item declares `label`");
    let dir = tempdir().expect("tempdir");
    for sibling in SIBLINGS {
        std::fs::copy(fixtures().join(sibling), dir.path().join(sibling)).expect("copy a sibling");
    }
    std::fs::write(dir.path().join("event_batch_item.scxml"), &reserved).expect("the item");
    let (ok, out) = run(
        &["check", "-l", "swift"],
        dir.path(),
        &SIBLINGS,
        "batch_reader.scxml",
        READER,
    );
    assert!(!ok, "a member Swift reserves is no member:\n{out}");
    for part in [
        "validation/reserved-code-identifier",
        "<data id=\\\"fileprivate\\\">",
        "'fileprivate' is a reserved word in swift",
    ] {
        assert!(out.contains(part), "expected `{part}` in:\n{out}");
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
/// `sce-build/src/forge/event_schema_check.rs`) and by the Swift row of the
/// table of what each backend lowers (`ComposedLowering::of`): Swift reads a
/// composed payload and holds a union (docs/adr/0092, section 7), so a read of
/// a payload that holds one, by the tag it tests, is accepted. A backend that
/// holds none refuses it by its own name
/// (`a_union_type_is_read_and_refused_by_name_where_it_is_not_lowered`).
#[test]
fn a_payload_that_holds_a_union_is_read_by_its_tag_on_swift() {
    let unions = repo_root().join("sce-build/tests/fixtures/union");
    let (ok, out) = run(
        &["check", "-l", "swift"],
        &unions,
        &SHAPES,
        "shapes_reader.scxml",
        SHAPES_READER,
    );
    assert!(ok, "Swift holds a union:\n{out}");
    assert!(
        !out.contains("has no swift lowering of its union"),
        "Swift lowers the union a payload holds:\n{out}"
    );
}
