// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A C11 machine sizes its slot of `_event.data` from the schemas of the events
//! it receives (docs/adr/0086, section 8.1): its header declares
//! `<SM>_DATA_LEN_DECLARED`, one more than the largest `DATA_MAX_LEN` of a
//! schema whose event a transition takes or whose record a host-run invocation
//! completes with, and `<SM>_DATA_LEN`, which every place of the machine that
//! holds an event's data is declared at. A schema the statechart imports and
//! never receives is no input.
//!
//! Each number below is `J` of the schema's payload by the table of section
//! 2.1, worked out by hand: the braces, each quoted key with its colon, each
//! value at its longest spelling (a text of `B` bytes at `2 + 6B`, a `uint8` at
//! 3, a `uint32` at 10), and a comma between each two members.

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

/// An event schema named `name` for the event `event`, whose fields are
/// `fields`, each its id with its type.
fn schema(name: &str, event: &str, fields: &[(&str, &str)]) -> String {
    let data: String = fields
        .iter()
        .map(|(id, ty)| format!("    <data id=\"{id}\" sce:type=\"{ty}\" sce:direction=\"in\"/>\n"))
        .collect();
    format!(
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       name="{name}" sce:kind="event-schema" sce:event-name="{event}">
  <datamodel>
{data}  </datamodel>
</scxml>"#
    )
}

/// A `sce-static` machine named `probe` that imports each of `imports` (the
/// file stem with the alias) and whose one state holds `body`.
fn machine(imports: &[(&str, &str)], body: &str) -> String {
    let imports: String = imports
        .iter()
        .map(|(stem, alias)| {
            format!("  <sce:import kind=\"event-schema\" src=\"{stem}.scxml\" as=\"{alias}\"/>\n")
        })
        .collect();
    format!(
        r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="probe">
{imports}  <datamodel>
    <data id="seen" sce:type="uint8" expr="0" sce:direction="out"/>
  </datamodel>
  <state id="idle">
{body}
  </state>
</scxml>
"#
    )
}

/// The header `sce-codegen generate -l c11` writes for the machine `doc`, with
/// each of `schemas` (its file stem with its text) beside it. `host_invoker`
/// names the host-run invoke type the host declares, if any.
fn header(doc: &str, schemas: &[(&str, String)], host_invoker: Option<&str>) -> String {
    let dir = tempdir().expect("tempdir");
    for (stem, text) in schemas {
        std::fs::write(dir.path().join(format!("{stem}.scxml")), text).expect("write a schema");
    }
    let path = dir.path().join("probe.scxml");
    std::fs::write(&path, doc).expect("write the machine");
    let out_dir = tempdir().expect("tempdir");
    let mut command = Command::new(sce_codegen_bin());
    command
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("generate")
        .arg(&path)
        .args(["-l", "c11"])
        .arg("-o")
        .arg(out_dir.path());
    if let Some(invoke_type) = host_invoker {
        command.args(["--host-invoker", invoke_type]);
    }
    let out = command.output().expect("invoke sce-codegen");
    assert!(
        out.status.success(),
        "the machine generates for C11:\n{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    std::fs::read_to_string(out_dir.path().join("probe_sm.h")).expect("the machine's header")
}

#[test]
fn the_slot_is_one_more_than_the_largest_schema_of_an_event_a_transition_takes() {
    let schemas = [
        // 2 + ("note": 7 + 26) + ("n": 4 + 3) + 1 comma = 43
        (
            "slot_a",
            schema("slot_a", "a.sent", &[("note", "string[4]"), ("n", "uint8")]),
        ),
        // 2 + ("text": 7 + 602) = 611
        (
            "slot_b",
            schema("slot_b", "b.sent", &[("text", "string[100]")]),
        ),
        // 2 + ("text": 7 + 6002) = 6011, past the other two: imported, and
        // received by no transition.
        (
            "slot_c",
            schema("slot_c", "c.sent", &[("text", "string[1000]")]),
        ),
    ];
    let doc = machine(
        &[
            ("slot_a", "SlotA"),
            ("slot_b", "SlotB"),
            ("slot_c", "SlotC"),
        ],
        r#"    <transition event="a.sent"><assign location="seen" expr="1"/></transition>
    <transition event="b.sent"><assign location="seen" expr="2"/></transition>"#,
    );
    let header = header(&doc, &schemas, None);
    assert!(
        header.contains("#define PROBE_DATA_LEN_DECLARED 612u"),
        "the slot holds `b.sent`'s 611 bytes and a NUL:\n{header}"
    );
    assert!(
        header.contains("`a.sent` (slot_a), which a transition takes: 43 bytes"),
        "`a.sent` is an input at its DATA_MAX_LEN:\n{header}"
    );
    assert!(
        header.contains("`b.sent` (slot_b), which a transition takes: 611 bytes"),
        "`b.sent` is an input at its DATA_MAX_LEN:\n{header}"
    );
    assert!(
        !header.contains("`c.sent`"),
        "a schema no transition takes is no input:\n{header}"
    );
    assert!(
        header.contains("char data[PROBE_DATA_LEN];"),
        "the event record holds its data in the machine's slot:\n{header}"
    );
    assert!(
        header.contains("(size_t)PROBE_DATA_LEN_DECLARED > (size_t)(SCE_MAX_DATA_LEN)"),
        "the slot is never smaller than the bound of the inputs no declaration sizes yet:\n{header}"
    );
}

#[test]
fn a_host_run_result_is_an_input_of_the_slot_though_no_transition_names_its_event() {
    // 2 + ("id": 5 + 10) + ("note": 7 + 98) + 1 comma = 123
    let schemas = [(
        "answer",
        schema(
            "answer",
            "answer.record",
            &[("id", "uint32"), ("note", "string[16]")],
        ),
    )];
    let doc = machine(
        &[("answer", "Answer")],
        r#"    <invoke type="x-app-host" id="ask" sce:result="Answer"/>
    <transition event="done.invoke.ask"><assign location="seen" expr="1"/></transition>"#,
    );
    let header = header(&doc, &schemas, Some("x-app-host"));
    assert!(
        header.contains("#define PROBE_DATA_LEN_DECLARED 124u"),
        "the slot holds the result's 123 bytes and a NUL:\n{header}"
    );
    assert!(
        header.contains(
            "`answer.record` (answer), which a host-run invocation completes with: 123 bytes"
        ),
        "the result is an input because the invocation completes with it:\n{header}"
    );
}

#[test]
fn a_machine_that_receives_no_schema_declares_a_slot_of_its_nul_alone() {
    let doc = machine(
        &[],
        r#"    <transition event="poke"><assign location="seen" expr="1"/></transition>"#,
    );
    let header = header(&doc, &[], None);
    assert!(
        header.contains("#define PROBE_DATA_LEN_DECLARED 1u"),
        "no input leaves the NUL alone:\n{header}"
    );
    assert!(
        header.contains("(docs/adr/0086, section 2.2); it receives none."),
        "the header says it receives no schema's event:\n{header}"
    );
}
