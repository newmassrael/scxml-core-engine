// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A value that may be absent is declared by its type, `optional<T>`
//! (docs/adr/0055, steps 1 and 2): a `sce-static` statechart's variable holds
//! one over a scalar, a value of an enum it imports or a record of an event
//! schema it imports, and an event schema's field over a scalar or an enum it
//! imports; the type is carried in the model and spelled back as written; every
//! other position refuses it by name; and every backend and the Interpreter
//! refuse a document that holds one, by name, until the step that lowers it on
//! that backend. `null` stands only where an optional value is declared, so a
//! slot that declares any other type refuses it.
//!
//! # What was wrong
//!
//! Measured in the code on 2026-10-21: no type said "may be absent", so a day an
//! application leaves empty had no spelling but a sentinel day or a second field,
//! and `slot_admits` admitted the `null` literal in every slot, so
//! `<data sce:type="int64" expr="null"/>` was judged fit.

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::forge::model::{AlgorithmValueType, OptionalElemType, SceType};
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

/// The documents a probe imports, copied beside it: an enum (`Status`) and an
/// event schema whose field holds a value of it (`Move`).
const SIBLINGS: [&str; 2] = ["enum_item_status.scxml", "schema_item_move.scxml"];

/// A `sce-static` statechart that imports `Status` and `Move` and declares
/// `data` (its `<data>` elements) and `body` (what its one state does).
fn statechart(data: &str, body: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="optional_probe">
  <sce:import kind="enum" src="enum_item_status.scxml" as="Status"/>
  <sce:import kind="event-schema" src="schema_item_move.scxml" as="Move"/>
  <datamodel>
    {data}
  </datamodel>
  <state id="idle">
    {body}
  </state>
</scxml>
"#
    )
}

/// `sce-codegen <args…> <file>` in a directory holding the probe's siblings and
/// `documents`, `file` being one of `documents`. Returns `(exit_ok, stderr +
/// stdout)`.
fn run(args: &[&str], documents: &[(&str, &str)], file: &str) -> (bool, String) {
    let dir = tempdir().expect("tempdir");
    for name in SIBLINGS {
        std::fs::copy(fixtures().join(name), dir.path().join(name)).expect("copy a sibling");
    }
    for (name, text) in documents {
        std::fs::write(dir.path().join(name), text).expect("write a document");
    }
    let out_dir = tempdir().expect("tempdir");
    let mut command = Command::new(sce_codegen_bin());
    command
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(args)
        .arg(dir.path().join(file));
    // `generate` and `lower` write a tree; `check` writes nothing.
    if matches!(args.first(), Some(&"generate") | Some(&"lower")) {
        command.arg("-o").arg(out_dir.path());
    }
    let out = command.output().expect("invoke sce-codegen");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout),
    )
}

/// `sce-codegen check -l rust` of the statechart `text` is refused with `code`,
/// and the refusal holds `needle`.
fn check_refuses(text: &str, code: &str, needle: &str) {
    let (ok, out) = run(
        &["check", "-l", "rust"],
        &[("probe.scxml", text)],
        "probe.scxml",
    );
    assert!(!ok, "expected a refusal with {code}:\n{out}");
    assert!(out.contains(code), "expected the code {code} in:\n{out}");
    assert!(out.contains(needle), "expected `{needle}` in:\n{out}");
}

/// The `generate` arguments of each of the six generated backends.
fn every_backend() -> Vec<Vec<&'static str>> {
    vec![
        vec!["generate", "-l", "rust"],
        vec!["generate", "-l", "kotlin"],
        vec!["generate", "-l", "python"],
        vec!["generate", "-l", "c11"],
        vec!["generate", "-l", "cpp"],
        vec![
            "generate",
            "-l",
            "go",
            "--go-module-prefix",
            "example.test/generated",
        ],
    ]
}

/// The refusal a backend gives a document that holds an optional value, before
/// the step that lowers one there (`StaticTarget::lowers_optional`).
const NOT_LOWERED: &str = "an optional value has no";

/// The backends whose step of docs/adr/0055 lowers a statechart's optional value,
/// by the language `generate -l` names: each generates what the others refuse.
/// A backend joins in the step that lowers the type on it.
const LOWERS_OPTIONAL: &[&str] = &["go", "python", "cpp"];

/// Whether the backend `args` (one of [`every_backend`]) lowers a statechart's
/// optional value.
fn lowers_optional(args: &[&str]) -> bool {
    args.get(2)
        .is_some_and(|lang| LOWERS_OPTIONAL.contains(lang))
}

// ── The type in the model ────────────────────────────────────────────────

#[test]
fn the_type_is_read_and_spelled_back_as_written() {
    for (written, elem) in [
        ("optional<int64>", OptionalElemType::Scalar(SceType::Int64)),
        ("optional<bool>", OptionalElemType::Scalar(SceType::Bool)),
        (
            "optional<string>",
            OptionalElemType::Scalar(SceType::String),
        ),
        (
            "optional<record:Move>",
            OptionalElemType::Record {
                alias: "Move".to_string(),
            },
        ),
    ] {
        let read = AlgorithmValueType::from_attr(written)
            .unwrap_or_else(|| panic!("`{written}` reads as a type"));
        assert_eq!(read, AlgorithmValueType::Optional { elem }, "{written}");
        assert_eq!(read.as_attr(), written, "spelled back as written");
        // An optional value is neither its element nor a list nor a record.
        assert!(read.scalar().is_none(), "{written} is no scalar");
        assert!(read.record_alias().is_none(), "{written} is no record");
        assert!(read.list_elem().is_none(), "{written} is no list");
        assert!(read.optional_elem().is_some(), "{written} is optional");
    }
    // What has no carrier for two absences, and a list, is no element.
    for refused in [
        "optional<optional<int64>>",
        "optional<list<int64>>",
        "optional<>",
    ] {
        assert!(
            AlgorithmValueType::from_attr(refused).is_none(),
            "`{refused}` is no type"
        );
    }
}

#[test]
fn the_forge_ast_spells_the_type_by_its_constructor() {
    let scalar = AlgorithmValueType::from_attr("optional<int64>").expect("a type");
    assert_eq!(
        serde_json::to_value(&scalar).expect("serialises"),
        serde_json::json!({ "optional": "int64" })
    );
    let record = AlgorithmValueType::from_attr("optional<record:Move>").expect("a type");
    assert_eq!(
        serde_json::to_value(&record).expect("serialises"),
        serde_json::json!({ "optional": { "record": "Move" } })
    );
    // A type that is not optional keeps the spelling it always had.
    let plain = AlgorithmValueType::from_attr("int64").expect("a type");
    assert_eq!(
        serde_json::to_value(&plain).expect("serialises"),
        serde_json::json!("int64")
    );
}

// ── A statechart's variable holds one, and no backend lowers it yet ──────

/// Each element a statechart's variable may hold reaches the backend, which
/// refuses it by name: the type was read, and the refusal is the lowering's,
/// not the parser's.
#[test]
fn a_variable_holds_each_element_and_is_refused_where_it_is_lowered() {
    for (data, spelled) in [
        (
            r#"<data id="due" sce:type="optional&lt;int64&gt;" expr="null"/>"#,
            "optional<int64>",
        ),
        (
            r#"<data id="ratio" sce:type="optional&lt;float64&gt;" expr="null"/>"#,
            "optional<float64>",
        ),
        (
            r#"<data id="flag" sce:type="optional&lt;bool&gt;" expr="null"/>"#,
            "optional<bool>",
        ),
        (
            r#"<data id="title" sce:type="optional&lt;string&gt;" sce:capacity="16" expr="null"/>"#,
            "optional<string>",
        ),
        (
            r#"<data id="frame" sce:type="optional&lt;bytes&gt;" sce:capacity="8" expr="null"/>"#,
            "optional<bytes>",
        ),
        (
            r#"<data id="status" sce:type="optional&lt;enum:Status&gt;" expr="null"/>"#,
            "optional<enum:Status>",
        ),
        (
            r#"<data id="pick" sce:type="optional&lt;record:Move&gt;" expr="null"/>"#,
            "optional<record:Move>",
        ),
    ] {
        let text = statechart(data, "");
        let (ok, out) = run(
            &["generate", "-l", "rust"],
            &[("probe.scxml", &text)],
            "probe.scxml",
        );
        assert!(
            !ok,
            "{spelled}: no backend lowers an optional value yet:\n{out}"
        );
        assert!(
            out.contains(NOT_LOWERED) && out.contains(spelled),
            "{spelled}: refused where it is lowered, naming the type:\n{out}"
        );
        assert!(
            out.contains("generate/unsupported-feature"),
            "{spelled}: refused as a feature this backend does not lower:\n{out}"
        );
    }
}

#[test]
fn every_backend_refuses_a_document_that_holds_an_optional_value_by_name() {
    let text = statechart(
        r#"<data id="due" sce:type="optional&lt;int64&gt;" expr="null"/>"#,
        "",
    );
    for args in every_backend() {
        let (ok, out) = run(&args, &[("probe.scxml", &text)], "probe.scxml");
        if lowers_optional(&args) {
            assert!(ok, "{args:?} lowers an optional value:\n{out}");
            continue;
        }
        assert!(!ok, "{args:?} must refuse the document:\n{out}");
        assert!(
            out.contains(NOT_LOWERED) && out.contains("optional<int64>"),
            "{args:?} refuses it by name:\n{out}"
        );
    }
}

/// The Interpreter's step lowers an optional value (docs/adr/0055): absent is
/// `null` in its data model, which the lowered document holds as written.
#[test]
fn the_interpreter_lowers_a_document_that_holds_an_optional_value() {
    let text = statechart(
        r#"<data id="due" sce:type="optional&lt;int64&gt;" expr="null"/>"#,
        "",
    );
    let (ok, out) = run(&["lower"], &[("probe.scxml", &text)], "probe.scxml");
    assert!(ok, "the ecmascript lowering lowers the document:\n{out}");
}

/// The same document with the variable a plain `int64` that starts at `0` is
/// generated: the refusal above is the optional value's and nothing else's.
#[test]
fn the_same_document_without_the_optional_value_is_generated() {
    let text = statechart(r#"<data id="due" sce:type="int64" expr="0"/>"#, "");
    let (ok, out) = run(
        &["generate", "-l", "rust"],
        &[("probe.scxml", &text)],
        "probe.scxml",
    );
    assert!(
        ok,
        "the document without an optional value generates:\n{out}"
    );
}

// ── What may stand inside, and what is refused ───────────────────────────

#[test]
fn an_optional_of_an_optional_is_refused_naming_what_may_stand_inside() {
    check_refuses(
        &statechart(
            r#"<data id="due" sce:type="optional&lt;optional&lt;int64&gt;&gt;" expr="null"/>"#,
            "",
        ),
        "validation/invalid-attribute",
        "optional<enum:Status>",
    );
}

#[test]
fn an_optional_list_is_refused_naming_what_may_stand_inside() {
    check_refuses(
        &statechart(
            r#"<data id="days" sce:type="optional&lt;list&lt;int64&gt;&gt;" expr="null"/>"#,
            "",
        ),
        "validation/invalid-attribute",
        "optional<record:Move>",
    );
}

#[test]
fn an_alias_the_document_does_not_import_is_refused() {
    for written in ["optional&lt;enum:Nope&gt;", "optional&lt;record:Nope&gt;"] {
        check_refuses(
            &statechart(
                &format!(r#"<data id="due" sce:type="{written}" expr="null"/>"#),
                "",
            ),
            "validation/invalid-attribute",
            "optional<int64>",
        );
    }
}

#[test]
fn a_list_holds_no_optional_element() {
    check_refuses(
        &statechart(
            r#"<data id="days" sce:type="list&lt;optional&lt;int64&gt;&gt;" sce:capacity="4"/>"#,
            "",
        ),
        "validation/attribute-rule-violated",
        "never an optional value",
    );
}

#[test]
fn an_optional_text_declares_its_bound() {
    check_refuses(
        &statechart(
            r#"<data id="title" sce:type="optional&lt;string&gt;" expr="null"/>"#,
            "",
        ),
        "scxml/static-datamodel-rule",
        "sce:capacity",
    );
}

/// An algorithm's slot and a codec's entry take no optional value yet
/// (docs/adr/0055, steps 8 and 12), and say so by name rather than offer the
/// scalars as if the type were misspelled. (An event schema's field takes one
/// from step 2, below.)
#[test]
fn every_other_position_refuses_an_optional_value_by_name() {
    let algorithm = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="due_or" version="1.0">
  <sce:signature>
    <sce:param name="due" type="optional&lt;int64&gt;"/>
    <sce:return type="int64"/>
  </sce:signature>
  <sce:body>
    <sce:return expr="0"/>
  </sce:body>
</scxml>
"#;
    let (ok, out) = run(
        &["check", "-l", "rust"],
        &[("due_or.scxml", algorithm)],
        "due_or.scxml",
    );
    assert!(!ok, "an optional value is refused there:\n{out}");
    assert!(
        out.contains("validation/attribute-rule-violated")
            && out.contains("this position does not take one"),
        "refused by name:\n{out}"
    );
}

// ── `null` stands only where an optional value is declared ───────────────

#[test]
fn null_is_no_initial_value_of_a_variable_that_is_not_optional() {
    check_refuses(
        &statechart(r#"<data id="count" sce:type="int64" expr="null"/>"#, ""),
        "expression/type-mismatch",
        "null",
    );
}

#[test]
fn null_is_not_assigned_to_a_variable_that_is_not_optional() {
    check_refuses(
        &statechart(
            r#"<data id="count" sce:type="int64" expr="0"/>"#,
            r#"<transition event="clear"><assign location="count" expr="null"/></transition>"#,
        ),
        "expression/type-mismatch",
        "null",
    );
}

/// A plain variable assigned a number still is: the refusal above is `null`'s.
#[test]
fn a_number_is_still_assigned_to_a_variable_of_a_number() {
    let text = statechart(
        r#"<data id="count" sce:type="int64" expr="0"/>"#,
        r#"<transition event="clear"><assign location="count" expr="1"/></transition>"#,
    );
    let (ok, out) = run(
        &["check", "-l", "rust"],
        &[("probe.scxml", &text)],
        "probe.scxml",
    );
    assert!(ok, "a number is assigned to an int64:\n{out}");
}

// ── An event schema's field (docs/adr/0055, step 2) ──────────────────────

/// The file name a probe imports the schema [`SCHEMA_DUE`] from.
const SCHEMA_DUE_FILE: &str = "schema_item_due.scxml";

/// An event schema whose fields may be absent — a day, a bounded text and a
/// value of an enum — beside a field every delivery carries.
const SCHEMA_DUE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="schema_item_due" sce:event-name="item.due">
  <sce:import src="enum_item_status.scxml" kind="enum" as="Status"/>
  <datamodel>
    <data id="id" sce:type="uint32" sce:direction="in"/>
    <data id="due" sce:type="optional&lt;int64&gt;" sce:direction="in"/>
    <data id="title" sce:type="optional&lt;string&gt;" sce:max-size="16" sce:direction="in"/>
    <data id="status" sce:type="optional&lt;enum:Status&gt;" sce:direction="in"/>
  </datamodel>
</scxml>
"#;

/// An event schema of one field, typed `written` and carrying `extra`.
fn schema_of(written: &str, extra: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="schema_probe" sce:event-name="item.probe">
  <sce:import src="enum_item_status.scxml" kind="enum" as="Status"/>
  <datamodel>
    <data id="probe" sce:type="{written}" sce:direction="in" {extra}/>
  </datamodel>
</scxml>
"#
    )
}

/// A `sce-static` statechart that imports [`SCHEMA_DUE`] as `Due`, declares
/// `data` and does `body` in its one state.
fn statechart_due(data: &str, body: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="optional_probe">
  <sce:import kind="enum" src="enum_item_status.scxml" as="Status"/>
  <sce:import kind="event-schema" src="{SCHEMA_DUE_FILE}" as="Due"/>
  <datamodel>
    {data}
  </datamodel>
  <state id="idle">
    {body}
  </state>
</scxml>
"#
    )
}

#[test]
fn a_schema_field_holds_an_optional_scalar_and_the_forge_ast_spells_it() {
    use sce_build::forge::model::{FieldType, ForgeDocument};
    let parsed = sce_build::forge::parser::parse_forge_with_imports(
        SCHEMA_DUE,
        sce_build::DocumentLabel::symmetric("schema_item_due"),
    )
    .expect("an optional field is read on an event schema")
    .expect("an event schema is a forge document");
    let ForgeDocument::EventSchema(schema) = &parsed.document else {
        panic!("expected an event schema, got {:?}", parsed.document.kind());
    };
    let declared = |id: &str| {
        schema
            .fields
            .iter()
            .find(|f| f.id == id)
            .map(|f| f.sce_type.clone())
            .unwrap_or_else(|| panic!("the field `{id}` is read"))
    };
    assert_eq!(declared("id"), FieldType::Scalar(SceType::Uint32));
    assert_eq!(
        declared("due"),
        FieldType::Optional {
            elem: SceType::Int64
        }
    );
    assert_eq!(
        declared("title"),
        FieldType::Optional {
            elem: SceType::String
        }
    );
    assert!(
        matches!(
            declared("status"),
            FieldType::Optional {
                elem: SceType::Enum(_)
            }
        ),
        "an optional enum field: {:?}",
        declared("status")
    );
    // The forge-AST spells an optional field as a variable of the same type
    // is spelled, and a field every delivery carries as it always was.
    let ast = serde_json::to_value(&parsed.document).expect("the document serialises");
    let fields = ast["fields"].as_array().expect("the schema's fields");
    let spelled = |id: &str| {
        fields
            .iter()
            .find(|f| f["id"] == id)
            .map(|f| f["sce_type"].clone())
            .unwrap_or_else(|| panic!("the field `{id}` is in the AST"))
    };
    assert_eq!(spelled("id"), serde_json::json!("uint32"));
    assert_eq!(spelled("due"), serde_json::json!({ "optional": "int64" }));
    assert_eq!(
        spelled("title"),
        serde_json::json!({ "optional": "string" })
    );
}

#[test]
fn every_backend_refuses_a_schema_with_an_optional_field_by_name() {
    for args in every_backend() {
        let (ok, out) = run(&args, &[(SCHEMA_DUE_FILE, SCHEMA_DUE)], SCHEMA_DUE_FILE);
        assert!(!ok, "{args:?} must refuse the schema's unit:\n{out}");
        assert!(
            out.contains(NOT_LOWERED) && out.contains("optional<int64>"),
            "{args:?} refuses it by name:\n{out}"
        );
    }
}

/// A machine that reads a payload of the schema, or holds a record of it,
/// would hold an optional value in the payload's lift and the record: each
/// backend and the Interpreter refuse it by name.
#[test]
fn every_backend_refuses_a_statechart_that_imports_a_schema_with_an_optional_field() {
    let text = statechart_due(
        r#"<data id="count" sce:type="int64" expr="0"/>"#,
        r#"<transition event="item.due"><assign location="count" expr="1"/></transition>"#,
    );
    let documents = [
        ("probe.scxml", text.as_str()),
        (SCHEMA_DUE_FILE, SCHEMA_DUE),
    ];
    for args in every_backend() {
        let (ok, out) = run(&args, &documents, "probe.scxml");
        if lowers_optional(&args) {
            assert!(ok, "{args:?} lowers the payload's optional field:\n{out}");
            continue;
        }
        assert!(!ok, "{args:?} must refuse the machine:\n{out}");
        assert!(
            out.contains(NOT_LOWERED)
                && out.contains("schema_item_due")
                && out.contains("optional<int64>"),
            "{args:?} refuses it by the schema's field:\n{out}"
        );
    }
    // The Interpreter reads the payload's optional field as it reads any field,
    // through its library (docs/adr/0055).
    let (ok, out) = run(&["lower"], &documents, "probe.scxml");
    assert!(
        ok,
        "the ecmascript lowering lowers the machine that imports the schema:\n{out}"
    );
}

/// An algorithm whose record is of the schema is judged, and refused where
/// each backend renders it, by name.
#[test]
fn every_backend_refuses_an_algorithm_whose_record_holds_an_optional_field() {
    let algorithm = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="due_id" version="1.0">
  <sce:import kind="event-schema" src="{SCHEMA_DUE_FILE}" as="Due"/>
  <sce:signature>
    <sce:param name="item" type="record:Due"/>
    <sce:return type="uint32"/>
  </sce:signature>
  <sce:body>
    <sce:return expr="item.id"/>
  </sce:body>
</scxml>
"#
    );
    let documents = [
        ("due_id.scxml", algorithm.as_str()),
        (SCHEMA_DUE_FILE, SCHEMA_DUE),
    ];
    for args in every_backend() {
        let (ok, out) = run(&args, &documents, "due_id.scxml");
        assert!(!ok, "{args:?} must refuse the algorithm:\n{out}");
        assert!(
            out.contains(NOT_LOWERED) && out.contains("record:Due"),
            "{args:?} refuses it by the record's field:\n{out}"
        );
    }
}

/// A host-run record has no carrier for an absent value until the step that
/// hands it one, so a typed `<invoke>` naming the schema is refused where the
/// typed interface is checked.
#[test]
fn a_typed_host_invoke_of_a_schema_with_an_optional_field_is_refused_by_name() {
    let text = statechart_due(
        "",
        r#"<invoke type="sce-host-due" id="ask" sce:result="Due"/>"#,
    );
    let (ok, out) = run(
        &["check", "-l", "rust"],
        &[("probe.scxml", &text), (SCHEMA_DUE_FILE, SCHEMA_DUE)],
        "probe.scxml",
    );
    assert!(!ok, "the typed invoke must be refused:\n{out}");
    assert!(
        out.contains("does not carry an optional value yet") && out.contains("optional<int64>"),
        "refused by the schema's field:\n{out}"
    );
}

/// `sce-codegen check -l rust` of the event schema [`schema_of`] `written`,
/// `extra`: `(exit_ok, stderr + stdout)`.
fn check_schema(written: &str, extra: &str) -> (bool, String) {
    run(
        &["check", "-l", "rust"],
        &[("schema_probe.scxml", &schema_of(written, extra))],
        "schema_probe.scxml",
    )
}

#[test]
fn an_optional_field_declares_no_quantity() {
    let (ok, out) = check_schema(
        "optional&lt;float64&gt;",
        r#"sce:quantity="celsius" sce:scale="0.5" sce:offset="0""#,
    );
    assert!(!ok, "a quantity on an optional field is refused:\n{out}");
    assert!(
        out.contains("validation/attribute-rule-violated") && out.contains("sce:quantity"),
        "refused on the attribute:\n{out}"
    );
    // The same quantity on a field every delivery carries is read.
    let (ok, out) = check_schema(
        "float64",
        r#"sce:quantity="celsius" sce:scale="0.5" sce:offset="0""#,
    );
    assert!(
        !out.contains("an optional field"),
        "a present field's quantity is no optional's refusal (ok={ok}):\n{out}"
    );
}

/// What may stand inside a schema field's `optional<…>` is a scalar or an
/// imported enum: a record, a nested optional and an alias nothing imports
/// are each refused naming that set.
#[test]
fn a_schema_field_refuses_what_may_not_stand_inside_naming_the_set() {
    for written in [
        "optional&lt;record:Move&gt;",
        "optional&lt;optional&lt;int64&gt;&gt;",
        "optional&lt;enum:Nope&gt;",
    ] {
        let (ok, out) = check_schema(written, "");
        assert!(!ok, "`{written}` is refused on a schema field:\n{out}");
        assert!(
            out.contains("validation/invalid-attribute")
                && out.contains("optional<int64>")
                && out.contains("optional<enum:Status>"),
            "`{written}` is refused naming the scalars and imported enums:\n{out}"
        );
    }
}
