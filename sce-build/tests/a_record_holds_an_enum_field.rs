// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// `datamodel="sce-static"`: a record whose event-schema has an enum field
// (docs/SCE_ACCEPTED_SUBSET.md §2.15).
//
// The field is held in the machine's own type for the enum document the schema
// imports (docs/adr/0054), and the field is judged as a variable of the enum is:
// it starts at a variant, is compared to and assigned values of its own enum, and
// takes no number. A record variable starts it at a variant, which the machine
// names through its own import of that document; a list of the records names
// none, and needs no import. The same holds through a loop's record item. Each
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

/// A schema that holds a composition: a list of `Gear` records, each holding
/// the enum field `speed`.
const FLEET: &str = r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="schema_fleet" sce:event-name="fleet.set">
  <sce:import kind="event-schema" src="schema_gear.scxml" as="Gear"/>
  <datamodel>
    <data id="gears" sce:type="list&lt;record:Gear&gt;[2]" sce:direction="in"/>
  </datamodel>
</scxml>
"##;

/// A statechart taking `fleet.set`, whose payload is a `Fleet`, with a `Mode`
/// variable `first`, a `Unit` variable `scale`, a `uint32` `total` and the given
/// transition `body`.
fn fleet_machine(body: &str) -> String {
    format!(
        r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static">
  <sce:import kind="event-schema" src="schema_fleet.scxml" as="Fleet"/>
  <sce:import kind="event-schema" src="schema_gear.scxml" as="Gear"/>
  <sce:import kind="enum" src="mode.scxml" as="Mode"/>
  <sce:import kind="enum" src="unit.scxml" as="Unit"/>
  <datamodel>
    <data id="first" sce:type="enum:Mode" expr="Mode.fast"/>
    <data id="scale" sce:type="enum:Unit" expr="Unit.metric"/>
    <data id="total" sce:type="uint32" expr="0"/>
  </datamodel>
  <state id="s">
    <transition event="fleet.set" type="internal">
      {body}
    </transition>
  </state>
</scxml>
"##
    )
}

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
    // The same variants as `mode.scxml`, in a document of its own: another enum.
    let mode_copy = MODE.replace("name=\"mode\"", "name=\"mode_copy\"");
    for (name, text) in [
        ("mode.scxml", MODE),
        ("mode_copy.scxml", mode_copy.as_str()),
        ("unit.scxml", UNIT),
        ("schema_gear.scxml", GEAR),
        ("schema_fleet.scxml", FLEET),
    ] {
        std::fs::write(dir.path().join(name), text).expect("write sibling");
    }
    let path = dir.path().join("probe.scxml");
    std::fs::write(&path, doc).expect("write probe");
    codegen(args, &path)
}

/// `sce-codegen` run with `args` on the document at `path`: whether it passed,
/// and what it wrote.
fn codegen(args: &[&str], path: &Path) -> (bool, String) {
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
fn a_record_variable_imports_the_enum_its_field_starts_at_a_variant_of() {
    // `gear` starts `speed` at a variant, which names the enum: a document that
    // does not import the enum document the schema imports is refused naming the
    // field, before `Mode.fast` is read as a name it does not declare.
    refused(
        &machine(false, "Mode.fast", ""),
        "which this document does not import",
    );
}

#[test]
fn the_enum_a_field_holds_is_the_document_the_schema_imports() {
    // `Mode` is imported under the alias the schema writes, but from another
    // document with the same variants: another enum, which the field does not hold.
    // The field holds the schema's own enum document, which this document does not
    // import, so the record variable that starts it at `Mode.fast` is refused, by
    // every backend alike, whether or not an algorithm is ever handed the record.
    let doc = machine(true, "Mode.fast", "").replacen(
        r#"src="mode.scxml" as="Mode""#,
        r#"src="mode_copy.scxml" as="Mode""#,
        1,
    );
    assert!(
        doc.contains("mode_copy.scxml"),
        "the probe imports Mode from the copy"
    );
    for args in [
        vec!["check"],
        vec!["check", "-l", "rust"],
        vec!["check", "-l", "c11"],
    ] {
        let (ok, out) = run(&args, &doc);
        assert!(!ok, "{args:?}: expected a refusal, got:\n{out}");
        assert!(
            out.contains("\"code\":\"scxml/static-datamodel-rule\""),
            "{args:?}: expected scxml/static-datamodel-rule:\n{out}"
        );
        assert!(
            out.contains("which this document does not import"),
            "{args:?}: the refusal names the rule:\n{out}"
        );
    }
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
fn an_enum_field_of_a_record_variable_is_carried_as_a_param() {
    // The field crosses as the name its enum declares for it, a string, on every
    // backend that lowers a `<send>`'s `<param>`.
    let body = r#"<send event="out"><param name="s" expr="gear.speed"/></send>"#;
    for args in [
        &["check"][..],
        &["check", "-l", "rust"],
        &["check", "-l", "kotlin"],
        &["check", "-l", "go"],
        &["check", "-l", "python"],
        &["check", "-l", "cpp"],
        &["check", "-l", "c11"],
    ] {
        let (ok, out) = run(args, &machine(true, "Mode.fast", body));
        assert!(ok, "{args:?}: an enum field of a record as a param:\n{out}");
    }
}

#[test]
fn an_enum_field_of_a_loop_item_is_no_param_yet() {
    // The value of a loop's record item has no lowering as a `<param>` here, so
    // it is refused with the other values that have no wire spelling, and not
    // left to stop a backend.
    refused(
        &machine(
            true,
            "Mode.fast",
            r#"<foreach array="all" item="g"><send event="out"><param name="s" expr="g.speed"/></send></foreach>"#,
        ),
        "an enum value of a loop's item",
    );
}

#[test]
fn a_list_of_records_holds_an_enum_field_the_document_does_not_import() {
    // No record variable of the schema: only the list, whose elements hold it. A
    // list starts empty and names no variant, so the field is held in the
    // machine's type for the enum document the schema imports, by every backend.
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
    let (ok, out) = run(&["check"], doc);
    assert!(ok, "a list of records whose enum is not imported:\n{out}");
    for language in emitting(ForgeKind::Statechart).map(|language| language.canonical_name()) {
        let (ok, out) = run(&["check", "-l", language], doc);
        assert!(
            ok,
            "{language}: a list of records whose enum is not imported:\n{out}"
        );
    }
}

#[test]
fn an_enum_field_of_a_payload_lists_element_is_a_value_of_its_enum() {
    // The payload holds a composition, so it has no record of entries: the
    // field of an element read through its index is typed by walking the
    // payload's type, and is a value of the enum the statechart imports the
    // schema's enum document under. It is assigned to a variable of that enum
    // and compared with its variant, by the judge and by every backend.
    let body = r#"
      <if cond="len(_event.data.gears) &gt; 0">
        <assign location="first" expr="_event.data.gears[0].speed"/>
      </if>
      <if cond="len(_event.data.gears) &gt; 1 &amp;&amp; _event.data.gears[1].speed === Mode.slow">
        <assign location="total" expr="total + _event.data.gears[1].count"/>
      </if>"#;
    let doc = fleet_machine(body);
    let (ok, out) = run(&["check"], &doc);
    assert!(ok, "an enum field of a payload list's element:\n{out}");
    for language in emitting(ForgeKind::Statechart).map(|language| language.canonical_name()) {
        let (ok, out) = run(&["check", "-l", language], &doc);
        assert!(
            ok,
            "{language}: an enum field of a payload list's element:\n{out}"
        );
    }
}

#[test]
fn an_enum_field_of_a_payload_lists_element_is_held_to_its_enum() {
    // Typed as the value of its enum it is, it is no value of another enum and
    // is compared with no number; a number read through the same element is no
    // value of the enum either.
    refused(
        &fleet_machine(r#"<assign location="scale" expr="_event.data.gears[0].speed"/>"#),
        "not of the enum `Unit`",
    );
    refused(
        &fleet_machine(r#"<assign location="first" expr="_event.data.gears[0].count"/>"#),
        "not of the enum `Mode`",
    );
    refused(
        &fleet_machine(
            r#"<if cond="_event.data.gears[0].speed === 0"><assign location="total" expr="1"/></if>"#,
        ),
        "a comparison of an enum value with anything but a value of the same enum",
    );
}

/// The test profile's shared imports, among them `order_line.scxml`, a record
/// holding the enum field `color` of `paint_color.scxml`, and
/// `payment_kind.scxml`, another enum.
fn profile_imports() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/test_profile/imports")
}

/// A message holding a list of order lines, the records of the profile's
/// `order_line.scxml` under `imports`.
fn line_batch(imports: &Path) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="line_batch" sce:event-name="line.batch">
  <sce:import kind="event-schema" src="{imports}/order_line.scxml" as="Line"/>
  <datamodel>
    <data id="lines" sce:type="list&lt;record:Line&gt;[2]" sce:direction="in"/>
  </datamodel>
</scxml>
"##,
        imports = imports.display()
    )
}

/// A port type that takes `line_batch`.
const LINE_BATCH_PORT: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-port-type" name="line_batch_port">
  <sce:import kind="event-schema" src="line_batch.scxml" as="LineBatch"/>
  <sce:message type="LineBatch" direction="in"/>
</scxml>
"##;

/// The template that matches any value of `line_batch`.
const LINE_BATCH_ANY: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-template" name="line_batch_any"
       sce:type="record:LineBatch">
  <sce:import kind="event-schema" src="line_batch.scxml" as="LineBatch"/>
  <sce:record-match>
    <sce:any-value field="lines"/>
  </sce:record-match>
</scxml>
"##;

/// A test component whose receive branch takes any `line_batch` on its port
/// `sut` and runs `body`, with the `Color` variable `first_color` and the
/// `PaymentKind` variable `paid_by`: the shape in which
/// `test_profile/received/parts_reader.scxml` keeps the color of the first line
/// of the message it took.
fn line_reader(imports: &Path, body: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-component" name="line_reader"
       datamodel="sce-static" initial="reading">
  <sce:import kind="test-port-type" src="line_batch_port.scxml" as="BatchPort"/>
  <sce:import kind="test-template" src="line_batch_any.scxml" as="AnyBatch"/>
  <sce:import kind="event-schema" src="line_batch.scxml" as="Batch"/>
  <sce:import kind="event-schema" src="{imports}/order_line.scxml" as="Line"/>
  <sce:import kind="enum" src="{imports}/paint_color.scxml" as="Color"/>
  <sce:import kind="enum" src="{imports}/payment_kind.scxml" as="PaymentKind"/>
  <sce:test-port id="sut" type="BatchPort"/>
  <datamodel>
    <data id="first_color" sce:type="enum:Color" expr="Color.red"/>
    <data id="paid_by" sce:type="enum:PaymentKind" expr="PaymentKind.cash"/>
  </datamodel>
  <state id="reading">
    <transition sce:receive="sut" sce:test-template="AnyBatch" target="done">
      {body}
      <sce:setverdict value="pass"/>
    </transition>
  </state>
  <final id="done"/>
</scxml>
"##,
        imports = imports.display()
    )
}

/// `sce-codegen` run with `args` on [`line_reader`] of `body`, beside the
/// message, the port type and the template it imports.
fn run_line_reader(args: &[&str], body: &str) -> (bool, String) {
    let dir = tempdir().expect("tempdir");
    let imports = profile_imports();
    for (name, text) in [
        ("line_batch.scxml", line_batch(&imports)),
        ("line_batch_port.scxml", LINE_BATCH_PORT.to_string()),
        ("line_batch_any.scxml", LINE_BATCH_ANY.to_string()),
    ] {
        std::fs::write(dir.path().join(name), text).expect("write sibling");
    }
    let path = dir.path().join("line_reader.scxml");
    std::fs::write(&path, line_reader(&imports, body)).expect("write component");
    codegen(args, &path)
}

#[test]
fn an_enum_field_of_a_received_lists_element_is_a_value_of_its_enum() {
    // A receive branch reads the message it took as `_event.data`, a record of
    // the schema its template matches, which holds a composition: the field of
    // an element of its list of records, read through an index, is typed by
    // walking that record's type, and is a value of the enum the component
    // imports the field's enum document under. It is assigned to a variable of
    // that enum, by the judge and by every backend a test component is written
    // on, as `test_profile/received/parts_reader.scxml` assigns it.
    let body = r#"<if cond="len(_event.data.lines) &gt; 0">
        <assign location="first_color" expr="_event.data.lines[0].color"/>
      </if>"#;
    let (ok, out) = run_line_reader(&["check"], body);
    assert!(ok, "an enum field of a received list's element:\n{out}");
    for language in emitting(ForgeKind::TestComponent).map(|language| language.canonical_name()) {
        let (ok, out) = run_line_reader(&["check", "-l", language], body);
        assert!(
            ok,
            "{language}: an enum field of a received list's element:\n{out}"
        );
    }
}

#[test]
fn an_enum_field_of_a_received_lists_element_is_held_to_its_enum() {
    // The same read is no value of another enum, and a number, whether read
    // through the same element or written, is no value of the enum.
    for (body, because) in [
        (
            r#"<assign location="paid_by" expr="_event.data.lines[0].color"/>"#,
            "not of the enum `PaymentKind`",
        ),
        (
            r#"<assign location="first_color" expr="_event.data.lines[0].quantity"/>"#,
            "not of the enum `Color`",
        ),
        (
            r#"<assign location="first_color" expr="2"/>"#,
            "not of the enum `Color`",
        ),
    ] {
        let (ok, out) = run_line_reader(&["check"], body);
        assert!(!ok, "expected a refusal ({because}) of {body}, got:\n{out}");
        assert!(
            out.contains(because),
            "the refusal of {body} must say {because:?}:\n{out}"
        );
    }
}
