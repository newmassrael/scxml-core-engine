// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A statechart's `record:R` variable of a record that holds a composition
//! (docs/adr/0087, "Decided while landing item 8", decision 10) is built where
//! it is declared by definite assignment: each field of one entry given by one
//! `<sce:set>`, an optional composed field starting absent and a list of no
//! lower count starting empty. A field with no value before something writes
//! one (a union, a record, a list of a lower count above 0) refuses the
//! variable by name, naming the repair `optional<record:R>`, and so does a
//! `<sce:set>` of a composed field, since no expression of the language builds
//! a list, a union or a record.
//!
//! `static_union.scxml`'s `board` is such a variable, and its scenario
//! (`scenarios/static_union.json`) replays its start, a guard's read of it, a
//! lift into it and its restore on every lane and the Interpreter.

mod common;

use std::path::{Path, PathBuf};

fn union_fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/union")
}

/// A record schema named `name` that imports the shape union and the rect
/// record, and declares `fields`.
fn schema(name: &str, fields: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="{name}" sce:event-name="{name}.kept">
  <sce:import kind="union" src="union_shape_minimal.scxml" as="Shape"/>
  <sce:import kind="event-schema" src="shape_rect_minimal.scxml" as="Rect"/>
  <datamodel>
{fields}
  </datamodel>
</scxml>
"#
    )
}

/// The record every accepted variable is of: a text, a number and an optional
/// text of one entry each, a list of rects of no lower count and a shape that
/// may be absent.
fn board() -> String {
    schema(
        "board",
        r#"    <data id="title" sce:type="string[8]" sce:direction="in"/>
    <data id="count" sce:type="uint8" sce:direction="in"/>
    <data id="note" sce:type="optional&lt;string[8]&gt;" sce:direction="in"/>
    <data id="rects" sce:type="list&lt;record:Rect&gt;[3]" sce:direction="in"/>
    <data id="pinned" sce:type="optional&lt;union:Shape&gt;" sce:direction="in"/>"#,
    )
}

/// A `sce-static` statechart that imports the shape union, its tag's enum, the
/// records of its cases and the record `board.scxml` as `Board`, and declares
/// `data` in its datamodel.
fn statechart(data: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="board_vars">
  <sce:import kind="enum" src="shape_kind.scxml" as="Kind"/>
  <sce:import kind="union" src="union_shape_minimal.scxml" as="Shape"/>
  <sce:import kind="event-schema" src="shape_radius_minimal.scxml" as="Radius"/>
  <sce:import kind="event-schema" src="shape_rect_minimal.scxml" as="Rect"/>
  <sce:import kind="event-schema" src="board.scxml" as="Board"/>
  <datamodel>
{data}
  </datamodel>
  <state id="idle"/>
</scxml>
"#
    )
}

/// `sce-codegen check` of `document` beside the union fixtures and `board`, the
/// text of `board.scxml`, under `args` (`-l <language>`, or none for the
/// Interpreter's check): whether it passed, and what it printed.
fn checked(args: &[&str], document: &str, board: &str) -> (bool, String) {
    let dir = tempfile::tempdir().expect("a scratch directory");
    for name in [
        "shape_kind.scxml",
        "shape_radius_minimal.scxml",
        "shape_rect_minimal.scxml",
        "union_shape_minimal.scxml",
    ] {
        std::fs::copy(union_fixtures().join(name), dir.path().join(name)).expect("copy a fixture");
    }
    std::fs::write(dir.path().join("board.scxml"), board).expect("write the record");
    let path = dir.path().join("board_vars.scxml");
    std::fs::write(&path, document).expect("write the statechart");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .arg("--workspace-root")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join(".."))
        .arg("--error-format=json")
        .arg("check")
        .args(args)
        .arg(&path)
        .output()
        .expect("invoke sce-codegen");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout),
    )
}

/// What `check` printed of `document`, which it refused, under the Interpreter's
/// check and Rust's alike: the rule is the judge's, the same for every lowering.
fn refusal(document: &str, board: &str) -> String {
    let mut said = String::new();
    for args in [&[][..], &["-l", "rust"][..]] {
        let (ok, text) = checked(args, document, board);
        assert!(!ok, "refused under {args:?}:\n{document}\n{text}");
        assert!(text.contains("scxml/static-datamodel-rule"), "{text}");
        said = text;
    }
    said
}

/// `text` holds each of `parts`.
fn holds(text: &str, parts: &[&str]) {
    for part in parts {
        assert!(text.contains(part), "expected `{part}` in:\n{text}");
    }
}

/// Pinned by `Judge::composed_record` and `SchemaField::start`
/// (`sce-build/src/forge/static_datamodel.rs`, `sce-build/src/forge/model.rs`):
/// a list whose type states a lower count above 0 has no value before
/// something writes one.
#[test]
fn a_record_that_holds_a_counted_list_is_refused_naming_the_optional_repair() {
    let counted = schema(
        "board",
        r#"    <data id="title" sce:type="string[8]" sce:direction="in"/>
    <data id="rects" sce:type="list&lt;record:Rect&gt;[1..3]" sce:direction="in"/>"#,
    );
    let text = refusal(
        &statechart(
            r#"    <data id="kept" sce:type="record:Board">
      <sce:set name="title" expr="'desk'"/>
    </data>"#,
        ),
        &counted,
    );
    holds(
        &text,
        &[
            "the field `rects` of record:Board",
            "a list of at least 1 element,",
            "optional<record:Board>",
        ],
    );
}

/// Pinned by `SchemaField::start`'s union and record arms: a union has no case
/// to start at, and a record field no value of its own without a writer.
#[test]
fn a_record_that_holds_a_union_or_a_record_is_refused_naming_the_optional_repair() {
    for (field, holds_what) in [
        (
            r#"    <data id="shape" sce:type="union:Shape" sce:direction="in"/>"#,
            "a union,",
        ),
        (
            r#"    <data id="shape" sce:type="record:Rect" sce:direction="in"/>"#,
            "a record,",
        ),
    ] {
        let text = refusal(
            &statechart(
                r#"    <data id="kept" sce:type="record:Board">
      <sce:set name="title" expr="'desk'"/>
    </data>"#,
            ),
            &schema(
                "board",
                &format!(
                    "    <data id=\"title\" sce:type=\"string[8]\" sce:direction=\"in\"/>\n{field}"
                ),
            ),
        );
        holds(
            &text,
            &[
                "the field `shape` of record:Board",
                holds_what,
                "optional<record:Board>",
            ],
        );
    }
}

/// Pinned by the `<sce:set>` loop of `Judge::composed_record`: an optional
/// composed field and a list each start where the record is declared, and no
/// expression builds either.
#[test]
fn a_set_of_a_composed_field_is_refused_by_name() {
    for (set, starts) in [
        (r#"<sce:set name="rects" expr="null"/>"#, "it starts empty"),
        (
            r#"<sce:set name="pinned" expr="null"/>"#,
            "it starts absent",
        ),
    ] {
        let text = refusal(
            &statechart(&format!(
                r#"    <data id="kept" sce:type="record:Board">
      <sce:set name="title" expr="'desk'"/>
      <sce:set name="count" expr="2"/>
      <sce:set name="note" expr="null"/>
      {set}
    </data>"#
            )),
            &board(),
        );
        holds(
            &text,
            &[
                "no expression of the language builds a list, a union or a record",
                starts,
            ],
        );
    }
}

/// Pinned by the `order_record_fields` call of `Judge::composed_record`: each
/// field of one entry is given, as a field of a record of entries is.
#[test]
fn a_field_of_one_entry_left_ungiven_is_refused() {
    let (ok, text) = checked(
        &["-l", "rust"],
        &statechart(
            r#"    <data id="kept" sce:type="record:Board">
      <sce:set name="title" expr="'desk'"/>
      <sce:set name="note" expr="null"/>
    </data>"#,
        ),
        &board(),
    );
    assert!(!ok, "a field of one entry is given:\n{text}");
    holds(
        &text,
        &[
            "every field of Board given by a <sce:set>",
            "`count` is not",
        ],
    );
}

/// Pinned by `Judge::record_starts`, which judges each given field of the
/// record as a record of entries' is: a text past its bound is refused.
#[test]
fn a_given_text_past_its_bound_is_refused() {
    let (ok, text) = checked(
        &["-l", "rust"],
        &statechart(
            r#"    <data id="kept" sce:type="record:Board">
      <sce:set name="title" expr="'a desk of nine'"/>
      <sce:set name="count" expr="2"/>
      <sce:set name="note" expr="null"/>
    </data>"#,
        ),
        &board(),
    );
    assert!(!ok, "a text past its bound is refused:\n{text}");
    holds(&text, &["title"]);
}

/// The variable every lowering takes: each field of one entry given, the list
/// and the union left to start empty and absent.
const BUILT: &str = r#"    <data id="kept" sce:type="record:Board" sce:direction="out">
      <sce:set name="title" expr="'desk'"/>
      <sce:set name="count" expr="2"/>
      <sce:set name="note" expr="null"/>
    </data>"#;

/// Pinned by `composed_record_start` (`sce-build/src/forge/static_lowering.rs`)
/// and each backend's `ComposedSpelling::record_of` and `empty_list`: every
/// backend whose row holds a record that holds a composition builds the
/// variable where it is declared, and so does the Interpreter.
#[test]
fn every_lowering_that_holds_the_record_builds_it_where_it_is_declared() {
    let document = statechart(BUILT);
    let mut held = 0;
    for (language, row) in common::composed_lowering::checked_backends() {
        if !row.records {
            continue;
        }
        held += 1;
        let (ok, text) = checked(&["-l", language], &document, &board());
        assert!(
            ok,
            "{language} builds the record where it is declared:\n{text}"
        );
    }
    assert!(held > 0, "some backend holds the record");
    let (ok, text) = checked(&[], &document, &board());
    assert!(
        ok,
        "the Interpreter builds the record where it is declared:\n{text}"
    );
}

/// The generated text of `static_union.scxml`, whose `board` is a `record:Board`
/// built where it is declared, on `language`.
fn static_union(language: sce_build::generator::Language) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/static_datamodel/static_union.scxml");
    sce_build::compile_scxml_lang_typed(
        path.to_str().expect("a utf-8 path"),
        &sce_build::find_template_dir_for(language),
        language,
    )
    .unwrap_or_else(|e| panic!("{language:?} lowers static_union: {e:?}"))
    .files
    .into_iter()
    .map(|(_, code)| code)
    .collect::<Vec<_>>()
    .join("\n")
}

/// Pinned by each backend's `record_of` and `empty_list` and by
/// `absent_value`: `board` starts at its record built from its fields in the
/// schema's order, its list of shapes empty and its pinned shape absent.
#[test]
fn each_backend_spells_the_start_of_the_built_record() {
    use sce_build::generator::Language;
    for (language, start) in [
        (
            Language::Rust,
            "shapes: ::sce_rust_runtime::portable::BoundedList::new(), pinned: None }",
        ),
        (Language::Kotlin, "shapes = emptyList(), pinned = null)"),
        (Language::Go, "{}, pinned: optional.None["),
        (Language::Python, "shapes=[], pinned=None)"),
        (Language::Swift, "shapes: [], pinned: nil)"),
        (Language::Cpp, "{}, /*pinned=*/std::optional<"),
        (
            Language::C11,
            "){ .len = 0u }, .pinned = sce_optional_record_",
        ),
    ] {
        holds(&static_union(language), &[start]);
    }
}

/// Pinned by `JsComposed::record_of` and `empty_list`: the Interpreter's
/// `<data>` of `board` is the plain object of its fields.
#[test]
fn the_interpreter_starts_the_built_record_at_the_object_of_its_fields() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/static_datamodel/static_union.scxml");
    let lowered =
        sce_build::forge::static_js::lower_file(path.to_str().expect("a utf-8 path"), Vec::new())
            .unwrap_or_else(|e| panic!("the Interpreter lowers static_union: {e:?}"));
    let document = roxmltree::Document::parse(&lowered).expect("well-formed");
    let start = document
        .descendants()
        .find(|node| node.has_tag_name("data") && node.attribute("id") == Some("board"))
        .and_then(|node| node.attribute("expr"))
        .expect("the <data> of `board` holds its start");
    holds(start, &["'title': ", "'shapes': [], 'pinned': null})"]);
}
