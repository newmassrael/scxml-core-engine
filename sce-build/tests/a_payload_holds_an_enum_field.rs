// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// `datamodel="sce-static"`: an event whose payload schema has an enum field
// (docs/SCE_ACCEPTED_SUBSET.md §2.15).
//
// The field is held in the machine's own type for the enum, so the machine
// imports the enum under the alias the schema writes, and `_event.data.<field>`
// is judged as a variable of the enum is: compared to and assigned values of its
// own enum, taking no number. Each refusal here is one an enum value would
// otherwise have walked past into generated code that does not compile.

use std::path::{Path, PathBuf};
use std::process::Command;

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
    let enums = if import_mode {
        r#"<sce:import kind="enum" src="mode.scxml" as="Mode"/>
  <sce:import kind="enum" src="unit.scxml" as="Unit"/>"#
    } else {
        ""
    };
    let variables = if import_mode {
        r#"<data id="mode" sce:type="enum:Mode" expr="Mode.fast"/>
    <data id="unit" sce:type="enum:Unit" expr="Unit.metric"/>"#
    } else {
        ""
    };
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
    for (name, text) in [
        ("mode.scxml", MODE),
        ("unit.scxml", UNIT),
        ("schema_gear.scxml", GEAR),
    ] {
        std::fs::write(dir.path().join(name), text).expect("write sibling");
    }
    let path = dir.path().join("probe.scxml");
    std::fs::write(&path, doc).expect("write probe");
    let out = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(args)
        .arg(&path)
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
    ] {
        let (ok, out) = run(args, &machine(true, body));
        assert!(ok, "{args:?}: an enum field of a payload:\n{out}");
    }
}

#[test]
fn the_enum_a_payload_field_holds_is_imported_under_the_alias_the_schema_writes() {
    // The judge reads the document and the lowering holds the field in the
    // machine's type for the enum, which a document that does not import it has
    // none of: refused naming the alias, in every language that lowers a payload
    // enum, and not left to stop the generator.
    let body = r#"<assign location="total" expr="_event.data.count"/>"#;
    for language in ["rust", "kotlin", "go", "python", "cpp"] {
        refused(
            &["check", "-l", language],
            &machine(false, body),
            "the enum `Mode`",
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
