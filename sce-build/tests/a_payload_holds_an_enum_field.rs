// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// `datamodel="sce-static"`: an event whose payload schema has an enum field
// (docs/SCE_ACCEPTED_SUBSET.md §2.15).
//
// The field is held in the machine's own type for the enum document the schema
// imports, whether or not the machine imports it too, and under whatever alias
// (docs/adr/0054); `_event.data.<field>` is judged as a variable of the enum is:
// compared to and assigned values of its own enum, taking no number. Each
// refusal here is one an enum value would otherwise have walked past into
// generated code that does not compile.

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::forge::codegen_matrix::emitting;
use sce_build::forge::model::ForgeKind;
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

const MODE: &str = r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="enum" name="mode" sce:underlying-type="uint8">
  <datamodel>
    <data id="variants">
      <sce:variant name="fast" value="0"/>
      <sce:variant name="slow" value="1"/>
    </data>
  </datamodel>
</scxml>
"##;

const UNIT: &str = r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="enum" name="unit" sce:underlying-type="uint8">
  <datamodel>
    <data id="variants">
      <sce:variant name="metric" value="0"/>
      <sce:variant name="imperial" value="1"/>
    </data>
  </datamodel>
</scxml>
"##;

/// The payload of `gear.set`: an enum field `speed` of `Mode` and a number
/// `count`.
const GEAR: &str = r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="schema_gear" sce:event-name="gear.set">
  <sce:import src="mode.scxml" kind="enum" as="Mode"/>
  <datamodel>
    <data id="speed" sce:type="enum:Mode" sce:direction="in"/>
    <data id="count" sce:type="uint8" sce:direction="in"/>
  </datamodel>
</scxml>
"##;

/// A statechart importing the schema and, when `import_mode`, the enums `Mode`
/// and `Unit`, holding a `Mode` variable `mode`, a `Unit` variable `unit` and a
/// `uint32` `total`, with the given transition `body` on `gear.set`.
fn machine(import_mode: bool, body: &str) -> String {
    if import_mode {
        machine_with(
            r#"<sce:import kind="enum" src="mode.scxml" as="Mode"/>
  <sce:import kind="enum" src="unit.scxml" as="Unit"/>"#,
            r#"<data id="mode" sce:type="enum:Mode" expr="Mode.fast"/>
    <data id="unit" sce:type="enum:Unit" expr="Unit.metric"/>"#,
            body,
        )
    } else {
        machine_with("", "", body)
    }
}

/// A statechart importing the schema and the enum imports `enums`, holding the
/// variables `variables` and a `uint32` `total`, with the given transition `body`
/// on `gear.set`.
fn machine_with(enums: &str, variables: &str, body: &str) -> String {
    format!(
        r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static">
  <sce:import kind="event-schema" src="schema_gear.scxml" as="Gear"/>
  {enums}
  <datamodel>
    {variables}
    <data id="total" sce:type="uint32" expr="0"/>
  </datamodel>
  <state id="s">
    <transition event="gear.set" type="internal">
      {body}
    </transition>
  </state>
</scxml>
"##
    )
}

fn run(args: &[&str], doc: &str) -> (bool, String) {
    let dir = tempdir().expect("tempdir");
    // The same variants as `mode.scxml`, in a document of its own: another enum.
    let mode_copy = MODE.replace("name=\"mode\"", "name=\"mode_copy\"");
    for (name, text) in [
        ("mode.scxml", MODE),
        ("mode_copy.scxml", mode_copy.as_str()),
        ("unit.scxml", UNIT),
        ("schema_gear.scxml", GEAR),
    ] {
        std::fs::write(dir.path().join(name), text).expect("write sibling");
    }
    let path = dir.path().join("probe.scxml");
    std::fs::write(&path, doc).expect("write probe");
    run_path(args, &path)
}

/// `sce-codegen <args…> <path>` with JSON diagnostics: `(exit_ok, stdout + stderr)`.
fn run_path(args: &[&str], path: &Path) -> (bool, String) {
    let out = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(args)
        .arg(path)
        .output()
        .expect("invoke sce-codegen");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout),
    )
}

fn refused(args: &[&str], doc: &str, because: &str) {
    let (ok, out) = run(args, doc);
    assert!(!ok, "expected a refusal ({because}), got:\n{out}");
    assert!(
        out.contains(because),
        "the refusal must say {because:?}:\n{out}"
    );
}

#[test]
fn an_enum_field_of_the_payload_is_compared_and_assigned_to_a_variable_of_its_enum() {
    let body = r#"
      <if cond="_event.data.speed === Mode.fast">
        <assign location="total" expr="total + _event.data.count"/>
      </if>
      <if cond="_event.data.speed !== mode">
        <assign location="mode" expr="_event.data.speed"/>
      </if>
      <assign location="mode" expr="_event.data.speed === Mode.slow ? Mode.fast : Mode.slow"/>"#;
    for args in [
        &["check"][..],
        &["check", "-l", "rust"],
        &["check", "-l", "kotlin"],
        &["check", "-l", "go"],
        &["check", "-l", "python"],
        &["check", "-l", "cpp"],
        &["check", "-l", "c11"],
    ] {
        let (ok, out) = run(args, &machine(true, body));
        assert!(ok, "{args:?}: an enum field of a payload:\n{out}");
    }
}

#[test]
fn a_payload_field_holds_the_enum_its_schema_imports_without_an_import_of_it() {
    // The document reads `count` and names no variant of `Mode`, so it imports
    // no enum: `speed` is held all the same, in the machine's type for the enum
    // document the schema imports (docs/adr/0054), in every language that lowers
    // a payload enum. Measured 2026-10-21, every one of them refused it ("this
    // document does not import it under that alias").
    let body = r#"<assign location="total" expr="_event.data.count"/>"#;
    for language in emitting(ForgeKind::Statechart).map(|language| language.canonical_name()) {
        let (ok, out) = run(&["check", "-l", language], &machine(false, body));
        assert!(
            ok,
            "{language}: a payload whose enum field the document does not import:\n{out}"
        );
        assert!(
            !out.contains("panicked"),
            "{language}: a payload enum field reached a builder without its type:\n{out}"
        );
    }
}

#[test]
fn a_payload_field_is_a_value_of_the_enum_under_the_alias_the_reader_imports_it() {
    // The document imports the schema's enum document under an alias of its own:
    // the field is a value of `Speed`, compared with its variants and assigned to a
    // variable of it, and not of the schema's `Mode`, which names nothing here.
    let doc = machine_with(
        r#"<sce:import kind="enum" src="mode.scxml" as="Speed"/>"#,
        r#"<data id="speed" sce:type="enum:Speed" expr="Speed.slow"/>"#,
        r#"
      <if cond="_event.data.speed === Speed.fast">
        <assign location="total" expr="total + 1"/>
      </if>
      <assign location="speed" expr="_event.data.speed"/>"#,
    );
    for args in [
        &["check"][..],
        &["check", "-l", "rust"],
        &["check", "-l", "kotlin"],
        &["check", "-l", "go"],
        &["check", "-l", "python"],
        &["check", "-l", "cpp"],
        &["check", "-l", "c11"],
    ] {
        let (ok, out) = run(args, &doc);
        assert!(ok, "{args:?}: the reader's own alias for the enum:\n{out}");
    }
}

#[test]
fn an_alias_the_reader_gives_another_enum_does_not_name_the_fields_enum() {
    // `Mode` here is another document with the same variants: the field is still a
    // value of the enum the schema imports, which this document does not import,
    // and a variant of the other enum is no value of it.
    let doc = machine_with(
        r#"<sce:import kind="enum" src="mode_copy.scxml" as="Mode"/>"#,
        "",
        r#"<if cond="_event.data.speed === Mode.fast"><assign location="total" expr="1"/></if>"#,
    );
    refused(
        &["check"],
        &doc,
        "a comparison of an enum value with anything but a value of the same enum",
    );
}

#[test]
fn a_document_that_names_no_variant_of_a_payload_enum_generates_on_every_backend() {
    // `static_payload_enum_unimported` handles `record_list.refused`, whose schema
    // holds two enums: it imports one under an alias of its own, and the other not
    // at all, and keeps the payload whole in a list of its records.
    let fixture = repo_root()
        .join("sce-build/tests/fixtures/static_datamodel/static_payload_enum_unimported.scxml");
    for language in emitting(ForgeKind::Statechart).map(|language| language.canonical_name()) {
        let out_dir = tempdir().expect("tempdir");
        let (ok, out) = run_path(
            &[
                "generate",
                "-l",
                language,
                "--go-module-prefix",
                "x/y",
                "-o",
                out_dir.path().to_str().expect("a UTF-8 path"),
            ],
            &fixture,
        );
        assert!(ok, "{language}: the fixture generates:\n{out}");
        assert!(
            out.contains("\"needs_script_engine\":false"),
            "{language}: the payload's enum fields need no engine:\n{out}"
        );
    }
}

#[test]
fn an_enum_field_of_the_payload_is_not_a_number() {
    refused(
        &["check"],
        &machine(
            true,
            r#"<assign location="total" expr="_event.data.speed"/>"#,
        ),
        "where a number or a bool is expected",
    );
    refused(
        &["check"],
        &machine(
            true,
            r#"<if cond="_event.data.speed === 0"><assign location="total" expr="1"/></if>"#,
        ),
        "a comparison of an enum value with anything but a value of the same enum",
    );
    refused(
        &["check"],
        &machine(
            true,
            r#"<assign location="total" expr="_event.data.speed + 1"/>"#,
        ),
        "an enum value used as an operand or an argument",
    );
}

#[test]
fn an_enum_field_of_the_payload_is_assigned_only_to_a_variable_of_its_own_enum() {
    refused(
        &["check"],
        &machine(
            true,
            r#"<assign location="unit" expr="_event.data.speed"/>"#,
        ),
        "not of the enum `Unit`",
    );
    refused(
        &["check"],
        &machine(
            true,
            r#"<if cond="_event.data.speed === Unit.metric"><assign location="total" expr="1"/></if>"#,
        ),
        "a comparison of an enum value with anything but a value of the same enum",
    );
}

#[test]
fn another_field_of_the_payload_is_still_a_number() {
    // The enum field is the one the judge knows holds an enum: `count` beside it
    // is read as the number it is.
    let (ok, out) = run(
        &["check", "-l", "rust"],
        &machine(
            true,
            r#"<assign location="total" expr="_event.data.count + 1"/>"#,
        ),
    );
    assert!(ok, "a number of the same payload:\n{out}");
}
