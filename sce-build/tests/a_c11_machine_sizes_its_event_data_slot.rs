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

/// The header and the source `sce-codegen generate -l c11` writes for the
/// machine `doc`, named `probe`, with each of `schemas` (its file stem with its
/// text) beside it, and `flags` after the language (the host's declared types).
fn generated(doc: &str, schemas: &[(&str, String)], flags: &[&str]) -> (String, String) {
    let dir = tempdir().expect("tempdir");
    for (stem, text) in schemas {
        std::fs::write(dir.path().join(format!("{stem}.scxml")), text).expect("write a schema");
    }
    let path = dir.path().join("probe.scxml");
    std::fs::write(&path, doc).expect("write the machine");
    let out_dir = tempdir().expect("tempdir");
    let out = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("generate")
        .arg(&path)
        .args(["-l", "c11"])
        .args(flags)
        .arg("-o")
        .arg(out_dir.path())
        .output()
        .expect("invoke sce-codegen");
    assert!(
        out.status.success(),
        "the machine generates for C11:\n{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let read = |name: &str| {
        std::fs::read_to_string(out_dir.path().join(name)).expect("a file of the machine")
    };
    (read("probe_sm.h"), read("probe_sm.c"))
}

/// The header of the machine `doc` ([`generated`]); `host_invoker` names the
/// host-run invoke type the host declares, if any.
fn header(doc: &str, schemas: &[(&str, String)], host_invoker: Option<&str>) -> String {
    let flags: Vec<&str> = host_invoker
        .map(|invoke_type| vec!["--host-invoker", invoke_type])
        .unwrap_or_default();
    generated(doc, schemas, &flags).0
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

// ── docs/adr/0086, section 8.5 (d): no buffer of the slot's size on a stack ──

/// `c` with every comment and every string and character literal blanked to
/// spaces, its newlines kept, so a brace or a bracket inside one is not read
/// as code.
fn code_only(c: &str) -> String {
    let chars: Vec<char> = c.chars().collect();
    let mut out = String::with_capacity(c.len());
    let mut i = 0usize;
    let blank = |ch: char| if ch == '\n' { '\n' } else { ' ' };
    while i < chars.len() {
        let ch = chars[i];
        let next = chars.get(i + 1).copied();
        if ch == '/' && next == Some('/') {
            while i < chars.len() && chars[i] != '\n' {
                out.push(' ');
                i += 1;
            }
        } else if ch == '/' && next == Some('*') {
            out.push_str("  ");
            i += 2;
            while i < chars.len() && !(chars[i] == '*' && chars.get(i + 1) == Some(&'/')) {
                out.push(blank(chars[i]));
                i += 1;
            }
            out.push_str("  ");
            i += 2;
        } else if ch == '"' || ch == '\'' {
            out.push(ch);
            i += 1;
            while i < chars.len() && chars[i] != ch {
                if chars[i] == '\\' {
                    out.push(' ');
                    i += 1;
                }
                if i < chars.len() {
                    out.push(blank(chars[i]));
                    i += 1;
                }
            }
            out.push(ch);
            i += 1;
        } else {
            out.push(ch);
            i += 1;
        }
    }
    out
}

/// The records of a machine that hold its slot of `_event.data`, by the end of
/// their type's name.
const SLOT_HOLDING_TYPES: [&str; 5] = [
    "_queued_event_t",
    "_scheduled_entry_t",
    "_inbox_input_t",
    "_host_request_t",
    "_scratch_t",
];

/// Whether the code-only line `line` declares a local array whose size names
/// `_DATA_LEN`, or a local record that holds the slot (a pointer to either is
/// not one).
fn declares_a_slot_sized_local(line: &str) -> bool {
    if line.contains("_DATA_LEN]") && !line.contains("(*") {
        return true;
    }
    SLOT_HOLDING_TYPES.iter().any(|ty| {
        line.match_indices(ty).any(|(at, _)| {
            let rest = line[at + ty.len()..].trim_start();
            rest.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_') && line.contains(';')
        })
    })
}

/// Every line of `c` inside a function's body that declares a local of the
/// slot's size ([`declares_a_slot_sized_local`]). A body is what a `{` opens at
/// the top level after a `)`, which a struct's, an enum's or an initializer's
/// `{` never follows.
fn slot_sized_locals(c: &str) -> Vec<String> {
    let code = code_only(c);
    let mut found = Vec::new();
    let mut depth = 0usize;
    let mut in_body = false;
    let mut last_significant = ' ';
    for (code_line, line) in code.lines().zip(c.lines()) {
        if in_body && depth > 0 && declares_a_slot_sized_local(code_line) {
            found.push(line.trim().to_string());
        }
        for ch in code_line.chars() {
            match ch {
                '{' => {
                    if depth == 0 {
                        in_body = last_significant == ')';
                    }
                    depth += 1;
                }
                '}' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        in_body = false;
                    }
                }
                _ => {}
            }
            if !ch.is_whitespace() {
                last_significant = ch;
            }
        }
    }
    found
}

/// The scanner reads a local array of the slot's size, and a local record that
/// holds the slot, as what it refuses; and a pointer to either, a member of a
/// struct and the slot's name inside a string or a comment as what it does
/// not.
#[test]
fn the_scanner_tells_a_slot_sized_local_from_a_pointer_and_a_member() {
    let c =
        "typedef struct probe_scratch {\n    char payload[PROBE_DATA_LEN];\n} probe_scratch_t;\n\
             SCE_SM_FN static void probe_send(probe_t *sm) {\n    \
             char *const _ed = sm->scratch.payload;\n    \
             char (*const rows)[PROBE_DATA_LEN] = sm->scratch.pairs;\n    \
             /* char _old[PROBE_DATA_LEN]; */\n    \
             probe_raise(sm, \"char x[PROBE_DATA_LEN];\");\n}\n\
             SCE_SM_FN static void probe_bad(probe_t *sm,\n        int n) {\n    \
             char _ed[PROBE_DATA_LEN];\n    \
             probe_queued_event_t held;\n}\n";
    assert_eq!(
        slot_sized_locals(c),
        vec![
            "char _ed[PROBE_DATA_LEN];".to_string(),
            "probe_queued_event_t held;".to_string()
        ]
    );
}

/// A machine with a script engine whose every kind of buffer of an event's size
/// is written: a send's payload of params and of a `<content expr>`, a delayed
/// send, the texts of a host-served send, and a host-run invocation's namelist,
/// params, request data and `<content expr>`.
const SCRIPTED: &str = r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="ecmascript" name="probe">
  <datamodel><data id="v" expr="1"/></datamodel>
  <state id="s">
    <onentry>
      <send event="e" target="#_internal"><param name="a" expr="v + 1"/></send>
      <send event="c" target="#_internal"><content expr="v + 1"/></send>
      <send event="d" delay="1s"><param name="q" expr="v + 2"/></send>
      <send event="h" type="x-sce-host"><param name="p" expr="'x' + v"/></send>
    </onentry>
    <invoke type="x-app-host" id="job" namelist="v"><param name="r" expr="v + 3"/></invoke>
    <transition event="go" target="t"/>
  </state>
  <state id="t">
    <invoke type="x-app-host" id="body"><content expr="'body' + v"/></invoke>
  </state>
</scxml>
"##;

/// A `sce-static` machine whose typed buffers are written: a send's payload, a
/// delayed send's, a host-served send's texts, a host-run invocation's request
/// with a typed result, and the lift and inject seam of an event whose text its
/// guard reads.
fn typed_machine() -> String {
    r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static" name="probe">
  <sce:import kind="event-schema" src="answer.scxml" as="Answer"/>
  <sce:import kind="event-schema" src="label.scxml" as="Label"/>
  <datamodel>
    <data id="count" sce:type="uint8" expr="0" sce:direction="out"/>
  </datamodel>
  <state id="s">
    <onentry>
      <send event="e" target="#_internal"><param name="n" expr="count"/></send>
      <send event="d" delay="1s"><param name="n" expr="count"/></send>
      <send event="h" type="x-sce-host"><param name="n" expr="count"/></send>
    </onentry>
    <invoke type="x-app-host" id="ask" sce:result="Answer"><param name="n" expr="count"/></invoke>
    <transition event="job.done" cond="_event.data.label == 'ok'"><assign location="count" expr="1"/></transition>
  </state>
</scxml>
"##
    .to_string()
}

/// No function of a generated C11 machine, of either kind, declares a local of
/// the slot's size: every such buffer is the machine's own storage, lent as a
/// view (docs/adr/0086, section 8.5 (d)).
#[test]
fn no_function_of_a_c11_machine_holds_a_buffer_of_the_slot_on_its_stack() {
    let hosts = [
        "--host-processor",
        "x-sce-host",
        "--host-invoker",
        "x-app-host",
    ];
    let typed_schemas = [
        (
            "answer",
            schema(
                "answer",
                "answer.record",
                &[("id", "uint32"), ("note", "string[16]")],
            ),
        ),
        (
            "label",
            schema("label", "job.done", &[("label", "string[8]")]),
        ),
    ];
    for (kind, doc, schemas) in [
        ("a script engine's", SCRIPTED.to_string(), &[][..]),
        ("a typed", typed_machine(), &typed_schemas[..]),
    ] {
        let (header, source) = generated(&doc, schemas, &hosts);
        for (file, text) in [("header", &header), ("source", &source)] {
            let locals = slot_sized_locals(text);
            assert!(
                locals.is_empty(),
                "{kind} machine's {file} holds a buffer of the slot on a stack:\n{}",
                locals.join("\n")
            );
        }
    }
}
