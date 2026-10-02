// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// `datamodel="sce-static"`: a record whose event-schema has an enum field
// (docs/SCE_ACCEPTED_SUBSET.md §2.15).
//
// The field is held in the machine's own type for the enum, so the machine
// imports the enum under the alias the schema writes, and the field is judged
// as a variable of the enum is: it starts at a variant, is compared to and
// assigned values of its own enum, and takes no number. The same holds through
// a loop's record item. Each refusal here is one an enum value would
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

/// A schema with an enum field `speed` of `Mode` and a number `count`.
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

/// A statechart importing the schema as `Gear` and, when `import_mode`, the
/// enum as `Mode`, with `gear` and `other` records, a list `all` of them, a
/// `uint32` `total` and the given transition `body`.
fn machine(import_mode: bool, set_speed: &str, body: &str) -> String {
    let mode = if import_mode {
        r#"<sce:import kind="enum" src="mode.scxml" as="Mode"/>
  <sce:import kind="enum" src="unit.scxml" as="Unit"/>"#
    } else {
        ""
    };
    format!(
        r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static">
  <sce:import kind="event-schema" src="schema_gear.scxml" as="Gear"/>
  {mode}
  <datamodel>
    <data id="gear" sce:type="record:Gear">
      <sce:set name="speed" expr="{set_speed}"/>
      <sce:set name="count" expr="1"/>
    </data>
    <data id="all" sce:type="list&lt;record:Gear&gt;" sce:capacity="3"/>
    <data id="total" sce:type="uint32" expr="0"/>
  </datamodel>
  <state id="s">
    <transition event="go" type="internal">
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

fn refused(doc: &str, because: &str) {
    let (ok, out) = run(&["check"], doc);
    assert!(!ok, "expected a refusal ({because}), got:\n{out}");
    assert!(
        out.contains(because),
        "the refusal must say {because:?}:\n{out}"
    );
}

#[test]
fn an_enum_field_is_compared_assigned_appended_and_read_through_a_loop_item() {
    let body = r#"
      <if cond="gear.speed === Mode.fast">
        <assign location="gear.speed" expr="Mode.slow"/>
      </if>
      <assign location="gear.speed" expr="gear.speed === Mode.slow ? Mode.fast : Mode.slow"/>
      <sce:append target="all" expr="gear"/>
      <foreach array="all" item="g">
        <if cond="g.speed !== Mode.slow">
          <assign location="total" expr="total + g.count"/>
        </if>
      </foreach>"#;
    for args in [
        &["check"][..],
        &["check", "-l", "rust"],
        &["check", "-l", "kotlin"],
    ] {
        let (ok, out) = run(args, &machine(true, "Mode.fast", body));
        assert!(ok, "{args:?}: an enum field of a record:\n{out}");
    }
}

#[test]
fn the_enum_a_field_holds_is_imported_under_the_alias_the_schema_writes() {
    refused(
        &machine(false, "Mode.fast", ""),
        "the document imports it under that alias",
    );
}

#[test]
fn an_enum_field_starts_and_is_assigned_at_a_variant_of_its_own_enum() {
    for value in ["1", "Unit.metric", "total", "gear.count"] {
        refused(&machine(true, value, ""), "not of the enum `Mode`");
        refused(
            &machine(
                true,
                "Mode.fast",
                &format!(r#"<assign location="gear.speed" expr="{value}"/>"#),
            ),
            "not of the enum `Mode`",
        );
    }
}

#[test]
fn an_enum_field_is_not_a_number() {
    refused(
        &machine(
            true,
            "Mode.fast",
            r#"<assign location="total" expr="gear.speed"/>"#,
        ),
        "where a number or a bool is expected",
    );
    refused(
        &machine(
            true,
            "Mode.fast",
            r#"<if cond="gear.speed === 0"><assign location="total" expr="1"/></if>"#,
        ),
        "a comparison of an enum value with anything but a value of the same enum",
    );
    refused(
        &machine(
            true,
            "Mode.fast",
            r#"<foreach array="all" item="g"><assign location="total" expr="g.speed + 1"/></foreach>"#,
        ),
        "an enum value used as an operand or an argument",
    );
}

#[test]
fn a_loop_items_enum_field_is_read_and_not_written() {
    refused(
        &machine(
            true,
            "Mode.fast",
            r#"<foreach array="all" item="g"><assign location="g.speed" expr="Mode.slow"/></foreach>"#,
        ),
        "which a <foreach> binds",
    );
}

#[test]
fn a_list_of_records_with_an_enum_field_needs_the_enum_too() {
    // No record variable of the schema: only the list, whose elements hold it.
    let doc = r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static">
  <sce:import kind="event-schema" src="schema_gear.scxml" as="Gear"/>
  <datamodel>
    <data id="all" sce:type="list&lt;record:Gear&gt;" sce:capacity="3"/>
  </datamodel>
  <state id="s"/>
</scxml>
"##;
    refused(doc, "the document imports it under that alias");
}
