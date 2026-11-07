// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A statechart's variable of a union (docs/adr/0092, section 6.1 item 5, and
//! its amendment of 2026-11-07): a sum type has no default value, so a
//! `union:U` variable is built where it is declared, one `<sce:set>` naming
//! the tag with a variant literal and one naming each field of that case, and
//! an `optional<union:U>` one starts absent. The Rust machine holds either in
//! its own sum of the union's records, and a saved state writes the sum as one
//! object, its tag beside its case's fields; every other backend refuses the
//! variable by its own name.

use std::path::{Path, PathBuf};

use sce_build::{compile_scxml_lang_typed, find_template_dir_for};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/union")
}

/// A `sce-static` statechart named `union_vars` that imports the shape union,
/// its tag's enum and the record of each case, and declares `data`.
fn statechart(data: &str) -> String {
    statechart_with(data, "")
}

/// [`statechart`] whose one state runs `entry` when it is entered.
fn statechart_with(data: &str, entry: &str) -> String {
    document("", data, entry)
}

/// [`statechart_with`] that imports the standard refusal record a lift keeps
/// why a text is no value in, and declares the text `said` (`string[64]`) and
/// the refusal `why` beside `data`.
fn lifting(data: &str, entry: &str) -> String {
    document(
        r#"
  <sce:import kind="event-schema" src="sce:std/lift/refusal.scxml" as="LiftRefusal"/>"#,
        &format!(
            r#"    <data id="said" sce:type="string[64]" expr="''"/>
    <data id="why" sce:type="optional&lt;record:LiftRefusal&gt;" expr="null"/>
{data}"#
        ),
        entry,
    )
}

/// The statechart of [`statechart_with`], with `imports` after its own.
fn document(imports: &str, data: &str, entry: &str) -> String {
    let state = if entry.is_empty() {
        "<state id=\"idle\"/>".to_string()
    } else {
        format!("<state id=\"idle\">\n    <onentry>\n{entry}\n    </onentry>\n  </state>")
    };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="union_vars">
  <sce:import kind="enum" src="shape_kind.scxml" as="Kind"/>
  <sce:import kind="union" src="union_shape.scxml" as="Shape"/>
  <sce:import kind="event-schema" src="shape_radius.scxml" as="Radius"/>
  <sce:import kind="event-schema" src="shape_rect.scxml" as="Rect"/>{imports}
  <datamodel>
{data}
  </datamodel>
  {state}
</scxml>
"#
    )
}

/// `document` written beside the union fixtures, in a scratch directory that
/// lives as long as the answer.
fn beside_fixtures(document: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("a scratch directory");
    for name in [
        "shape_kind.scxml",
        "shape_radius.scxml",
        "shape_rect.scxml",
        "union_shape.scxml",
    ] {
        std::fs::copy(fixtures().join(name), dir.path().join(name)).expect("copy a fixture");
    }
    let path = dir.path().join("union_vars.scxml");
    std::fs::write(&path, document).expect("write the statechart");
    (dir, path)
}

/// `sce-codegen check -l <language>` of `document`: whether it passed, and
/// what it printed.
fn checked(language: &str, document: &str) -> (bool, String) {
    let (_dir, path) = beside_fixtures(document);
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .arg("--workspace-root")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join(".."))
        .arg("--error-format=json")
        .args(["check", "-l", language])
        .arg(&path)
        .output()
        .expect("invoke sce-codegen");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout),
    )
}

/// The Rust machine of `document`.
fn rust_machine(document: &str) -> String {
    let (dir, path) = beside_fixtures(document);
    generate_rust(dir, &path)
}

/// The Rust machine of `document` beside a union document whose text is
/// `union` in place of the fixture's.
fn rust_machine_with_union(document: &str, union: &str) -> String {
    let (dir, path) = beside_fixtures(document);
    std::fs::write(dir.path().join("union_shape.scxml"), union).expect("write the union");
    generate_rust(dir, &path)
}

/// The Rust machine `path` names, generated while `_dir` holds its files.
fn generate_rust(_dir: tempfile::TempDir, path: &Path) -> String {
    let language = sce_build::generator::Language::Rust;
    let output = compile_scxml_lang_typed(
        path.to_str().expect("a utf-8 path"),
        &find_template_dir_for(language),
        language,
    )
    .unwrap_or_else(|e| panic!("Rust holds a union variable: {e:?}"));
    output
        .files
        .into_iter()
        .find(|(name, _)| name == "union_vars_sm.rs")
        .map(|(_, code)| code)
        .expect("the machine's Rust is generated")
}

/// A refusal printed by `check` holds each of `parts`.
fn refused_with(text: &str, parts: &[&str]) {
    for part in parts {
        assert!(text.contains(part), "expected `{part}` in:\n{text}");
    }
}

const CIRCLE: &str = r#"    <data id="shape" sce:type="union:Shape" sce:direction="out">
      <sce:set name="kind" expr="Kind.circle"/>
      <sce:set name="radius" expr="4"/>
    </data>"#;
const DOT: &str = r#"    <data id="mark" sce:type="union:Shape">
      <sce:set name="kind" expr="Kind.dot"/>
    </data>"#;
const LAST: &str = r#"    <data id="last" sce:type="optional&lt;union:Shape&gt;" expr="null"/>"#;

const UNION: &str = "UnionVarsUnionShapeUnion";

/// Pinned by `union_variable_field` and `RustComposed::union_value`
/// (`sce-build/src/forge/static_lowering.rs`,
/// `sce-build/src/forge/composed_lowering.rs`): the case its tag names,
/// holding the record its other `<sce:set>`s build; an empty case alone; an
/// optional one absent.
#[test]
fn a_rust_machine_starts_a_union_variable_at_the_case_it_builds() {
    let code = rust_machine(&statechart(&format!("{CIRCLE}\n{DOT}\n{LAST}")));
    for line in [
        format!("let shape: {UNION} = {UNION}::Circle(UnionVarsRadiusRecord {{ radius: "),
        format!("let mark: {UNION} = {UNION}::Dot;"),
        format!("let last: Option<{UNION}> = None;"),
        format!("pub enum {UNION} {{"),
        // Lent to a host by reference, as a record that owns a buffer is.
        format!("pub fn shape(&self) -> &{UNION} {{"),
    ] {
        assert!(code.contains(&line), "expected `{line}` in:\n{code}");
    }
}

/// Pinned by the `static_unions` loop of
/// `tools/codegen/templates/rust/state_machine.rs.jinja2` and by
/// `saved_shape`: the union is saved as its tag beside its case's fields and
/// read back by its tag first, and the saved shape names the union's document
/// and each case's record.
#[test]
fn a_saved_state_holds_a_union_variable_as_its_tag_beside_its_case() {
    let code = rust_machine(&statechart(&format!("{CIRCLE}\n{DOT}\n{LAST}")));
    for line in [
        format!("impl ::sce_rust_runtime::saved_state::SavedValue for {UNION} {{"),
        "Self::Circle(record) => ::sce_rust_runtime::saved_state::union_to_saved(\"kind\", \
         \"circle\", Some(::sce_rust_runtime::saved_state::SavedValue::to_saved(record))),"
            .to_string(),
        "Self::Dot => ::sce_rust_runtime::saved_state::union_to_saved(\"kind\", \"dot\", None),"
            .to_string(),
        "match ::sce_rust_runtime::saved_state::union_tag(value, what, \"kind\")? {".to_string(),
        "\"rect\" => Ok(Self::Rect(::sce_rust_runtime::saved_state::SavedValue::from_saved(\
         value, what)?)),"
            .to_string(),
        "impl ::sce_rust_runtime::saved_state::SavedValue for UnionVarsRectRecord {".to_string(),
    ] {
        assert!(code.contains(&line), "expected `{line}` in:\n{code}");
    }
    // A case's record moves the shape: the same statechart over a union whose
    // `rect` holds the radius record is bound to another shape, though no
    // line of the statechart changed.
    let shape = |code: &str| -> String {
        let at = code
            .find("SavedShape {\n        first: \"")
            .expect("the machine is saved");
        let end = code[at..].find("\",\n").expect("the first shape ends");
        code[at..at + end].to_string()
    };
    let document = statechart(CIRCLE);
    let held = rust_machine(&document);
    let moved = rust_machine_with_union(
        &document,
        &std::fs::read_to_string(fixtures().join("union_shape.scxml"))
            .expect("the union fixture")
            .replace(
                r#"<sce:case variant="rect" record="Rect"/>"#,
                r#"<sce:case variant="rect" record="Radius"/>"#,
            ),
    );
    assert_ne!(
        shape(&held),
        shape(&moved),
        "a case's record moves the shape"
    );
    assert_eq!(
        shape(&held),
        shape(&rust_machine(&document)),
        "one document, one shape"
    );
}

/// Pinned by `Judge::union_variable` (`sce-build/src/forge/static_datamodel.rs`)
/// and the amendment of 2026-11-07: a bare union has no value to start at.
#[test]
fn a_union_variable_that_builds_no_case_is_refused_naming_both_repairs() {
    let (ok, text) = checked(
        "rust",
        &statechart(r#"    <data id="shape" sce:type="union:Shape"/>"#),
    );
    assert!(!ok, "a union variable with no case is refused:\n{text}");
    refused_with(
        &text,
        &[
            "scxml/static-datamodel-rule",
            "a union has no default value",
            "optional<union:Shape>",
            "<sce:set name=\\\"kind\\\" expr=\\\"Kind.<variant>\\\"/>",
        ],
    );
}

/// Pinned by `Judge::union_variable`: the tag is a variant literal, since the
/// build must know the case to know which fields follow.
#[test]
fn a_union_variable_whose_tag_is_no_variant_literal_is_refused() {
    let data = r#"    <data id="which" sce:type="enum:Kind" expr="Kind.circle"/>
    <data id="shape" sce:type="union:Shape">
      <sce:set name="kind" expr="which"/>
      <sce:set name="radius" expr="4"/>
    </data>"#;
    let (ok, text) = checked("rust", &statechart(data));
    assert!(!ok, "a tag that is no literal is refused:\n{text}");
    refused_with(
        &text,
        &[
            "scxml/static-datamodel-rule",
            "a variant of its enum written as a literal",
            "dot, circle, rect",
        ],
    );
}

/// Pinned by `Judge::union_variable`: an empty case is its tag alone, and a
/// case's record is built whole, each field once.
#[test]
fn a_union_variable_builds_exactly_the_fields_of_its_case() {
    let stray = r#"    <data id="shape" sce:type="union:Shape">
      <sce:set name="kind" expr="Kind.dot"/>
      <sce:set name="radius" expr="4"/>
    </data>"#;
    let (ok, text) = checked("rust", &statechart(stray));
    assert!(!ok, "a field after an empty case is refused:\n{text}");
    refused_with(
        &text,
        &["scxml/static-datamodel-rule", "is empty, its tag alone"],
    );
    let missing = r#"    <data id="shape" sce:type="union:Shape">
      <sce:set name="kind" expr="Kind.rect"/>
      <sce:set name="width" expr="4"/>
    </data>"#;
    let (ok, text) = checked("rust", &statechart(missing));
    assert!(!ok, "a case missing a field is refused:\n{text}");
    refused_with(
        &text,
        &["validation/attribute-rule-violated", "`height` is not"],
    );
}

/// Pinned by `Judge::optional_union_variable`: an optional union starts absent.
#[test]
fn an_optional_union_variable_starts_absent() {
    let (ok, text) = checked(
        "rust",
        &statechart(r#"    <data id="last" sce:type="optional&lt;union:Shape&gt;" expr="3"/>"#),
    );
    assert!(
        !ok,
        "an optional union that starts present is refused:\n{text}"
    );
    refused_with(
        &text,
        &[
            "scxml/static-datamodel-rule",
            "an optional union starts absent",
        ],
    );
    let (ok, text) = checked("rust", &statechart(LAST));
    assert!(ok, "an optional union that starts absent is held:\n{text}");
}

/// Pinned by `ComposedLowering::of` (`sce-build/src/forge/event_schema_check.rs`):
/// a backend that does not hold a union variable refuses it by its own name.
#[test]
fn a_backend_that_holds_no_union_variable_refuses_it_by_name() {
    let (ok, text) = checked("rust", &statechart(CIRCLE));
    assert!(ok, "Rust holds a union variable:\n{text}");
    let (ok, text) = checked("kotlin", &statechart(CIRCLE));
    assert!(!ok, "Kotlin holds no union variable yet:\n{text}");
    refused_with(
        &text,
        &[
            "generate/unsupported-feature",
            "<data id=\\\"shape\\\">: `union:Shape` in a statechart's variable",
            "kotlin lowering of its union",
        ],
    );
}

/// Pinned by `Judge::union_assign` (`sce-build/src/forge/static_datamodel.rs`):
/// the tag is never written alone, since it would leave the old case's fields
/// under the new tag (docs/adr/0092, section 6.1 item 6).
#[test]
fn the_tag_of_a_union_is_never_written_alone() {
    let (ok, text) = checked(
        "rust",
        &statechart_with(
            CIRCLE,
            r#"      <assign location="shape.kind" expr="Kind.rect"/>"#,
        ),
    );
    assert!(!ok, "a tag written alone is refused:\n{text}");
    refused_with(
        &text,
        &[
            "scxml/static-datamodel-rule",
            "the tag of a union is never written alone",
            "a whole assignment of `shape`",
        ],
    );
}

/// Pinned by the read judged at an `<assign>`'s location and by
/// `union_field_place` (`sce-build/src/forge/static_lowering.rs`): a field of
/// a case is written where a test proves the case, through the case's mutable
/// proven read, and refused where none does.
#[test]
fn a_field_of_a_union_variable_is_written_where_its_case_is_proven() {
    let proven = r#"      <if cond="shape.kind === Kind.circle">
        <assign location="shape.radius" expr="9"/>
      </if>"#;
    let code = rust_machine(&statechart_with(CIRCLE, proven));
    assert!(
        code.contains("self.shape.proven_circle_mut().radius = "),
        "expected the write through the proven case in:\n{code}"
    );
    let (ok, text) = checked(
        "rust",
        &statechart_with(
            CIRCLE,
            r#"      <assign location="shape.radius" expr="9"/>"#,
        ),
    );
    assert!(
        !ok,
        "a write where no test proves the case is refused:\n{text}"
    );
    refused_with(&text, &["expression/union-read-unproven", "radius"]);
}

/// Pinned by `Judge::union_assign` and `whole_union_copy`: a union variable
/// takes another value of its union whole, and `null` only where it may be
/// absent; nothing in an expression makes a union.
#[test]
fn a_union_variable_is_put_whole_from_a_value_of_its_union() {
    let whole = format!("{CIRCLE}\n{DOT}\n{LAST}");
    let code = rust_machine(&statechart_with(
        &whole,
        r#"      <assign location="mark" expr="shape"/>
      <assign location="last" expr="shape"/>
      <assign location="last" expr="null"/>"#,
    ));
    for line in [
        "self.mark = self.shape.clone();",
        "self.last = Some(self.shape.clone());",
        "self.last = None;",
    ] {
        assert!(code.contains(line), "expected `{line}` in:\n{code}");
    }
    let (ok, text) = checked(
        "rust",
        &statechart_with(&whole, r#"      <assign location="shape" expr="null"/>"#),
    );
    assert!(!ok, "a union that is never absent takes no null:\n{text}");
    refused_with(
        &text,
        &["scxml/static-datamodel-rule", "which is never absent"],
    );
    let (ok, text) = checked(
        "rust",
        &statechart_with(&whole, r#"      <assign location="shape" expr="3"/>"#),
    );
    assert!(!ok, "a number is no union:\n{text}");
    refused_with(
        &text,
        &[
            "scxml/static-datamodel-rule",
            "nothing in an expression makes a union",
        ],
    );
}

/// Pinned by `Lifted::Union` and `RustTarget::lift`
/// (`sce-build/src/forge/static_lowering.rs`) and the runtime's `lift_value`:
/// a lift whose schema is a union reads the whole text as one value of it, by
/// the reader a delivery's field of the union is read by (its tag first), and
/// puts it into the variable whole, present where the variable is optional
/// (docs/adr/0092, section 6.1 item 5).
#[test]
fn a_lift_into_a_union_variable_reads_the_text_whole_by_its_tag() {
    let code = rust_machine(&lifting(
        &format!("{CIRCLE}\n{LAST}"),
        r#"      <sce:lift text="said" schema="Shape" location="last" refusal="why"/>
      <sce:lift text="said" schema="Shape" location="shape" refusal="why"/>"#,
    ));
    for line in [
        "::sce_rust_runtime::event_payload::lift_value(&",
        ", \"Shape\", 1, ",
        "|v| v.union(\"kind\", |v| ",
        "Ok(sce_lifted) => { self.last = Some(sce_lifted);",
        "Ok(sce_lifted) => { self.shape = sce_lifted;",
    ] {
        assert!(code.contains(line), "expected `{line}` in:\n{code}");
    }
    assert!(
        !code.contains("lift_record(&"),
        "a union is read whole, never field by field as a record:\n{code}"
    );
}

/// Pinned by `Judge::lift` (`sce-build/src/forge/static_datamodel.rs`): a lift
/// whose schema is a union replaces a variable of that union, and names it in
/// the refusal of any other location.
#[test]
fn a_lift_into_a_union_replaces_only_a_variable_of_that_union() {
    let (ok, text) = checked(
        "rust",
        &lifting(
            &format!("{CIRCLE}\n    <data id=\"count\" sce:type=\"uint32\" expr=\"0\"/>"),
            r#"      <sce:lift text="said" schema="Shape" location="count" refusal="why"/>"#,
        ),
    );
    assert!(!ok, "a lift into a number is refused:\n{text}");
    refused_with(
        &text,
        &[
            "validation/lift-location",
            "names a variable of uint32",
            "union:Shape or optional<union:Shape>",
        ],
    );
    let (ok, text) = checked(
        "rust",
        &lifting(
            CIRCLE,
            r#"      <sce:lift text="said" schema="Shape" location="shape" refusal="why"/>"#,
        ),
    );
    assert!(ok, "a lift into a variable of the union is held:\n{text}");
}

/// Pinned by `ComposedLowering::of` (`sce-build/src/forge/event_schema_check.rs`):
/// a backend that holds no union variable refuses a lift into one by naming the
/// variable, before it is asked for the lift.
#[test]
fn a_backend_that_holds_no_union_refuses_a_lift_into_one_by_name() {
    let (ok, text) = checked(
        "kotlin",
        &lifting(
            LAST,
            r#"      <sce:lift text="said" schema="Shape" location="last" refusal="why"/>"#,
        ),
    );
    assert!(!ok, "Kotlin lifts into no union yet:\n{text}");
    refused_with(
        &text,
        &[
            "generate/unsupported-feature",
            "<data id=\\\"last\\\">: `union:Shape` in a statechart's variable",
            "kotlin lowering of its union",
        ],
    );
}
