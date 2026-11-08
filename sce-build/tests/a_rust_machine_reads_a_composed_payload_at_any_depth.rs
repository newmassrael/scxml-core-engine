// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A Rust `sce-static` machine reads the payload of an event whose schema holds
//! a composition, at any depth (docs/adr/0086, section 6), the first backend to
//! lower one: its payload struct holds each field in the machine column of
//! section 7.1, each record it names is declared once, the lift reads each
//! field by one runtime call per constructor (section 4), the inject seam
//! writes each as its JSON (section 3.1), and the statechart's reads lower to
//! the machine's own types.
//!
//! # What is held
//!
//! * The payload struct, the record a list of it holds, and the enum a list and
//!   a record hold, each in the machine's own type.
//! * The lift: a list of records that hold a list, a list of lists, an optional
//!   element, a list that may be absent, a top-level text held to its bound,
//!   and the defaults of the list the second revision added.
//! * The reads: `<foreach>` over a list the payload holds and over a list an
//!   enclosing loop's item holds, an index into a list of lists, a field of an
//!   element read through its index.
//! * What Rust does not lower yet stays refused by name on Rust too: a
//!   statechart's variable of a record that holds a list (`static_list_payload`).

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

/// The Rust machine of [`READER`], written beside the batch schemas.
fn reader_machine() -> String {
    let dir = tempdir().expect("tempdir");
    for sibling in SIBLINGS {
        std::fs::copy(fixtures().join(sibling), dir.path().join(sibling)).expect("copy a sibling");
    }
    let path = dir.path().join("batch_reader.scxml");
    std::fs::write(&path, READER).expect("write the reader");
    let language = sce_build::generator::Language::Rust;
    let output = compile_scxml_lang_typed(
        path.to_str().expect("a utf-8 path"),
        &find_template_dir_for(language),
        language,
    )
    .unwrap_or_else(|e| panic!("Rust reads a composed payload: {e:?}"));
    output
        .files
        .into_iter()
        .find(|(name, _)| name == "batch_reader_sm.rs")
        .map(|(_, code)| code)
        .expect("the machine's Rust is generated")
}

/// `code` holds each of `lines`.
fn holds(code: &str, lines: &[&str]) {
    for line in lines {
        assert!(code.contains(line), "expected `{line}` in:\n{code}");
    }
}

/// The members of the struct `code` declares under `header`, up to the brace
/// that closes it.
fn struct_body<'c>(code: &'c str, header: &str) -> &'c str {
    let start = code
        .find(header)
        .unwrap_or_else(|| panic!("expected `{header}` in:\n{code}"));
    let body = &code[start + header.len()..];
    &body[..body.find("\n}").expect("the struct closes")]
}

/// Pinned by `RustComposed::record_type`, `RustComposed::list_type`,
/// `RustComposed::optional_type`, `RustComposed::text_type` and
/// `RustComposed::bytes_type` (`sce-build/src/forge/composed_lowering.rs`), and
/// by the declaration of each record a payload names
/// (`declare_composed_payload`, `sce-build/src/forge/static_lowering.rs`).
#[test]
fn the_payload_is_held_in_the_machine_types_each_record_declared_once() {
    let code = reader_machine();
    holds(
        &code,
        &[
            "pub struct BatchReaderBatchSentPayload {",
            "    pub items: ::sce_rust_runtime::portable::BoundedList<BatchReaderItemRecord, 3>,",
            "    pub grid: ::sce_rust_runtime::portable::BoundedList<::sce_rust_runtime::portable::BoundedList<u8, 2>, 2>,",
            "    pub levels: ::sce_rust_runtime::portable::BoundedList<BatchReaderSignalLevelEnum, 2>,",
            "    pub maybe: ::sce_rust_runtime::portable::BoundedList<Option<u16>, 2>,",
            "    pub spare: Option<::sce_rust_runtime::portable::BoundedList<u8, 2>>,",
            "    pub note: ::sce_rust_runtime::portable::SceString<4>,",
            "    pub late: ::sce_rust_runtime::portable::BoundedList<u8, 2>,",
            "pub struct BatchReaderItemRecord {",
            "    pub label: ::sce_rust_runtime::portable::SceString<8>,",
            "    pub level: BatchReaderSignalLevelEnum,",
            "    pub due: Option<i64>,",
            "    pub blob: ::sce_rust_runtime::portable::SceBytes<4>,",
            "    pub tags: ::sce_rust_runtime::portable::BoundedList<::sce_rust_runtime::portable::SceString<8>, 4>,",
        ],
    );
    for header in [
        "pub struct BatchReaderBatchSentPayload {",
        "pub struct BatchReaderItemRecord {",
    ] {
        let body = struct_body(&code, header);
        assert!(
            !body.contains("Vec<"),
            "every list the machine holds names its bound in its type:\n{body}"
        );
    }
    assert_eq!(
        code.matches("pub struct BatchReaderItemRecord {").count(),
        1,
        "a record is declared once"
    );
}

/// Pinned by `RustComposed::read_list`, `RustComposed::read_record`,
/// `RustComposed::read_text` and `RustComposed::read_optional`, and by
/// `composed_defaults_wire` (`sce-build/src/forge/schema_revision.rs`): each
/// field read by one runtime call per constructor, a text held to its bound at
/// the top level too, a list the second revision added read as empty.
#[test]
fn the_lift_reads_each_field_by_its_type_at_every_depth() {
    let code = reader_machine();
    holds(
        &code,
        &[
            r#"PayloadFields::decode_with_defaults(data, "{\"late\":[]}")?"#,
            r#"items: fields.field("items").and_then(|v| v.list::<_, 3>(|v| v.record(None, |r| Ok(BatchReaderItemRecord { id: r.field("id").and_then(|v| v.unsigned::<u32>())?, "#,
            r#"tags: r.field("tags").and_then(|v| v.list::<_, 4>(|v| v.text::<8>()))? }))))?,"#,
            r#"blob: r.field("blob").and_then(|v| v.bytes::<4>())?, "#,
            r#"grid: fields.field("grid").and_then(|v| v.list::<_, 2>(|v| v.list::<_, 2>(|v| v.unsigned::<u8>())))?,"#,
            r#"maybe: fields.field("maybe").and_then(|v| v.list::<_, 2>(|v| v.optional(|v| v.unsigned::<u16>())))?,"#,
            r#"spare: fields.field("spare").and_then(|v| v.optional(|v| v.list::<_, 2>(|v| v.unsigned::<u8>())))?,"#,
            r#"note: fields.field("note").and_then(|v| v.text::<4>())?,"#,
        ],
    );
}

/// Pinned by `RustComposed::write_list`, `RustComposed::write_record` and
/// `RustComposed::write_optional`: the inject seam writes the payload it is
/// handed as the JSON a lift reads back.
#[test]
fn the_inject_seam_writes_each_field_as_its_json() {
    let code = reader_machine();
    let p = "::sce_rust_runtime::event_payload";
    holds(
        &code,
        &[
            &format!("let data = {p}::record_json(vec![(\"items\", {p}::list_json(&payload.items, |h| {p}::record_json(vec![(\"id\", h.id.to_string()),"),
            &format!("(\"grid\", {p}::list_json(&payload.grid, |h| {p}::list_json(h, |h| h.to_string())))"),
            &format!("(\"spare\", {p}::optional_json(payload.spare.as_ref(), |h| {p}::list_json(h, |h| h.to_string())))"),
            &format!("(\"note\", {p}::quote(&payload.note))"),
        ],
    );
}

/// Pinned by the loop over a list a place holds (`walked_place`,
/// `sce-build/src/forge/static_lowering.rs`) and by the read of a list element
/// by reference (`rust_element_by_reference`, `rust_list_operand`,
/// `sce-build/src/forge/expr.rs`).
#[test]
fn the_reads_lower_at_any_depth() {
    let code = reader_machine();
    holds(
        &code,
        &[
            "for (sce_position_of_i, row) in ev.items.clone().into_iter().enumerate()",
            "for word in row.tags.clone()",
            "for cells in ev.grid.clone()",
            "for n in cells.clone()",
            "sce_forge_runtime::algorithm::at_ref(&ev.items, i)?.tags",
            "sce_forge_runtime::algorithm::at(sce_forge_runtime::algorithm::at_ref(&ev.grid, 1)?, 0)?",
            "sce_forge_runtime::algorithm::at(&ev.maybe, 0)?",
        ],
    );
}

/// `sce-codegen <args> <file>` over `document` written beside the batch
/// schemas, and whether it succeeded, with what it said.
fn run(args: &[&str], name: &str, document: &str) -> (bool, String) {
    let dir = tempdir().expect("tempdir");
    for sibling in SIBLINGS {
        std::fs::copy(fixtures().join(sibling), dir.path().join(sibling)).expect("copy a sibling");
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
/// Rust takes the read, and a backend that does not lower it refuses it by its
/// own name.
#[test]
fn rust_takes_the_read_and_another_backend_refuses_it_by_name() {
    let (ok, out) = run(&["check", "-l", "rust"], "batch_reader.scxml", READER);
    assert!(ok, "Rust reads a composed payload:\n{out}");
    let (ok, out) = run(&["check", "-l", "cpp"], "batch_reader.scxml", READER);
    assert!(!ok, "C++ does not read a composed payload yet:\n{out}");
    assert!(out.contains("generate/unsupported-feature"), "{out}");
    assert!(
        out.contains("a statechart's read of `_event.data` has no cpp lowering of its list"),
        "{out}"
    );
}

/// Pinned by the call of `composed_uses_unlowered` in `render_rust`
/// (`sce-build/src/generator.rs`), which asks `ComposedLowering::of` on every
/// profile, and by `RustComposed::list_type` (docs/adr/0086, section 7.4): a
/// machine built without an allocator reads a composed payload as one built
/// with one does, every list held inline in a `BoundedList` whose bound is its
/// type's, and with no lift of the JSON wire that profile does not have.
#[test]
fn a_machine_without_an_allocator_reads_the_payload_in_bounded_types() {
    let (ok, out) = run(
        &["check", "-l", "rust", "--no-std"],
        "batch_reader.scxml",
        READER,
    );
    assert!(ok, "no_std reads a composed payload:\n{out}");
    let code = generated_no_std(READER);
    holds(
        &code,
        &[
            "    pub items: ::sce_rust_runtime::portable::BoundedList<BatchReaderItemRecord, 3>,",
            "    pub tags: ::sce_rust_runtime::portable::BoundedList<::sce_rust_runtime::portable::SceString<8>, 4>,",
            "sce_forge_runtime::algorithm::at_ref(&ev.items, i)?.tags",
        ],
    );
    for header in [
        "pub struct BatchReaderBatchSentPayload {",
        "pub struct BatchReaderItemRecord {",
    ] {
        let body = struct_body(&code, header);
        assert!(
            !body.contains("Vec<"),
            "no list needs an allocator:\n{body}"
        );
    }
    assert!(
        !code.contains("PayloadFields::decode"),
        "no_std has no JSON wire to lift a payload from:\n{code}"
    );
}

/// The Rust machine `sce-codegen generate -l rust --no-std` writes of
/// `document`, written beside the batch schemas as `batch_reader.scxml`.
fn generated_no_std(document: &str) -> String {
    let dir = tempdir().expect("tempdir");
    for sibling in SIBLINGS {
        std::fs::copy(fixtures().join(sibling), dir.path().join(sibling)).expect("copy a sibling");
    }
    let path = dir.path().join("batch_reader.scxml");
    std::fs::write(&path, document).expect("write the document");
    let out = Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("generate")
        .arg(&path)
        .arg("-l")
        .arg("rust")
        .arg("--no-std")
        .arg("-o")
        .arg(dir.path())
        .output()
        .expect("invoke sce-codegen");
    assert!(
        out.status.success(),
        "sce-codegen generate --no-std failed:\n{}{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    std::fs::read_to_string(dir.path().join("batch_reader_sm.rs")).expect("the machine's Rust")
}

/// Pinned by `whole_list_copy` (`sce-build/src/forge/static_lowering.rs`) and
/// `copier` (`sce-build/src/forge/composed_lowering.rs`): a list a payload holds
/// is taken whole into a list variable of a larger bound, a copy of the one
/// type the machine holds both in.
#[test]
fn a_list_is_taken_whole_into_a_variable_whose_type_it_fits() {
    let dir = tempdir().expect("tempdir");
    for sibling in SIBLINGS
        .iter()
        .chain(["static_composed_payload.scxml"].iter())
    {
        std::fs::copy(fixtures().join(sibling), dir.path().join(sibling)).expect("copy a sibling");
    }
    let path = dir.path().join("static_composed_payload.scxml");
    let language = sce_build::generator::Language::Rust;
    let output = compile_scxml_lang_typed(
        path.to_str().expect("a utf-8 path"),
        &find_template_dir_for(language),
        language,
    )
    .unwrap_or_else(|e| panic!("Rust lowers the fixture: {e:?}"));
    let code = output
        .files
        .into_iter()
        .find(|(name, _)| name == "static_composed_payload_sm.rs")
        .map(|(_, code)| code)
        .expect("the machine's Rust is generated");
    // The variable holds a list of a larger bound than the payload's, so the
    // copy widens it by type, which rustc checks again (docs/adr/0086,
    // section 7.4).
    holds(
        &code,
        &[
            "late: ::sce_rust_runtime::portable::BoundedList<u8, 4>,",
            "self.late = ::sce_rust_runtime::portable::BoundedList::widened_map(&ev.late, |v| *v);",
        ],
    );
    // A backend that reads no composed payload refuses the fixture by its own
    // name.
    let text =
        std::fs::read_to_string(fixtures().join("static_composed_payload.scxml")).expect("fixture");
    let (ok, out) = run(
        &["check", "-l", "cpp"],
        "static_composed_payload.scxml",
        &text,
    );
    assert!(!ok, "C++ reads no composed payload yet:\n{out}");
    assert!(out.contains("has no cpp lowering"), "{out}");
}

/// What Rust does not lower yet stays refused by name on Rust too: the fixture's
/// `kept`, a variable of records that hold a list (docs/adr/0086, section 7.3).
#[test]
fn a_variable_of_a_record_that_holds_a_list_stays_refused_on_rust() {
    let text =
        std::fs::read_to_string(fixtures().join("static_list_payload.scxml")).expect("the fixture");
    let (ok, out) = run(&["check", "-l", "rust"], "static_list_payload.scxml", &text);
    assert!(!ok, "no backend holds such a variable yet:\n{out}");
    assert!(out.contains("generate/unsupported-feature"), "{out}");
    assert!(
        out.contains("the field `tags`") && out.contains("a statechart's variable"),
        "{out}"
    );
}
