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

/// What `sce-codegen generate -l c11` answers for the machine `doc`, named
/// `probe`, with each of `schemas` (its file stem with its text) beside it, and
/// `flags` after the language (the host's declared types): the run's output
/// and the directory it wrote into.
fn run_codegen(
    doc: &str,
    schemas: &[(&str, String)],
    flags: &[&str],
) -> (std::process::Output, tempfile::TempDir) {
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
    (out, out_dir)
}

/// The header and the source `sce-codegen generate -l c11` writes for the
/// machine `doc` ([`run_codegen`]), which must generate.
fn generated(doc: &str, schemas: &[(&str, String)], flags: &[&str]) -> (String, String) {
    let (out, out_dir) = run_codegen(doc, schemas, flags);
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
    // 2 + ("id": 5 + 10) + ("note": 7 + 386) + 1 comma = 411, past the 255
    // bytes of the host's refusal the machine also holds.
    let schemas = [(
        "answer",
        schema(
            "answer",
            "answer.record",
            &[("id", "uint32"), ("note", "string[64]")],
        ),
    )];
    let doc = machine(
        &[("answer", "Answer")],
        r#"    <invoke type="x-app-host" id="ask" sce:result="Answer"/>
    <transition event="done.invoke.ask"><assign location="seen" expr="1"/></transition>"#,
    );
    let header = header(&doc, &schemas, Some("x-app-host"));
    assert!(
        header.contains("#define PROBE_DATA_LEN_DECLARED 412u"),
        "the slot holds the result's 411 bytes and a NUL:\n{header}"
    );
    assert!(
        header.contains(
            "`answer.record` (answer), which a host-run invocation completes with: 411 bytes"
        ),
        "the result is an input because the invocation completes with it:\n{header}"
    );
}

/// docs/adr/0086, section 8.6 (b): a host may refuse a host-run invocation's
/// start with a text of its own, which the machine raises whole. The host ABI
/// bounds that text at 255 bytes, so a machine that runs a host-run invocation
/// holds a reason of 255 bytes, and checks the bound against its header.
#[test]
fn a_host_refusal_is_a_reason_of_the_bound_the_host_abi_declares() {
    // The result is 123 bytes (see the test above, with a note of 16), so the
    // refusal's 255 bytes are what the slot holds.
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
    let (header, source) = generated(&doc, &schemas, &["--host-invoker", "x-app-host"]);
    let longest = longest_reason(&header);
    assert!(
        longest >= 255,
        "the host's refusal is a reason of up to 255 bytes:\n{header}"
    );
    assert!(
        header.contains(&format!("#define PROBE_DATA_LEN_DECLARED {}u", longest + 1)),
        "the slot holds the refusal, past the result's 123 bytes:\n{header}"
    );
    assert!(
        source.contains(
            "SCE_STATIC_ASSERT((size_t)(SCE_HOST_INVOKE_REFUSAL_MAX_LEN) <= (size_t)255u,"
        ),
        "the machine checks the bound its slot was sized by:\n{source}"
    );
    let plain = machine(
        &[],
        r#"    <transition event="poke"><assign location="seen" expr="1"/></transition>"#,
    );
    let (_, plain_source) = generated(&plain, &[], &[]);
    assert!(
        !plain_source.contains("SCE_HOST_INVOKE_REFUSAL_MAX_LEN"),
        "a machine that runs no host-run invocation relies on no refusal bound:\n{plain_source}"
    );
}

/// docs/adr/0086, section 8.6 (b): the lift of a typed payload quotes the
/// sentence a payload reader refused a field with. `sce/event_payload.h`
/// publishes the longest such sentence, 120 bytes, so the reason is measured
/// at its row's text plus 120, and the machine checks the bound.
#[test]
fn a_lift_reason_is_measured_at_its_text_plus_the_published_refusal_bound() {
    let schemas = [("lifted", schema("lifted", "a.sent", &[("n", "uint8")]))];
    let doc = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="ecmascript" name="probe">
  <sce:import kind="event-schema" src="lifted.scxml" as="Lifted"/>
  <state id="idle">
    <transition event="a.sent" cond="_event.data.n === 1" target="done"/>
    <transition event="error.execution" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#;
    let (header, source) = generated(doc, &schemas, &[]);
    let text = "`a.sent` payload: 'n' ";
    assert!(
        source.contains(&format!(
            "snprintf(_message, _message_cap, \"{text}%s\", _refusal);"
        )),
        "the lift writes the field's refusal through the table, quoting the sentence:\n{source}"
    );
    let measured = text.len() as u64 + 120;
    assert!(
        longest_reason(&header) >= measured,
        "the lift's reason is measured at {measured} bytes:\n{header}"
    );
    assert!(
        source.contains("SCE_STATIC_ASSERT((size_t)(SCE_PAYLOAD_REFUSAL_MAX_LEN) <= (size_t)120u,"),
        "the machine checks the bound its slot was sized by:\n{source}"
    );
}

/// The longest reason the header of a machine says it raises (docs/adr/0086,
/// section 8.6 (b)), in bytes.
fn longest_reason(header: &str) -> u64 {
    let (_, after) = header
        .split_once("the longest it\n   raises is an input too: ")
        .unwrap_or_else(|| panic!("the header states the longest reason:\n{header}"));
    let (bytes, _) = after
        .split_once(" bytes.")
        .unwrap_or_else(|| panic!("the longest reason is a number of bytes:\n{header}"));
    bytes
        .parse()
        .unwrap_or_else(|_| panic!("`{bytes}` is a number of bytes:\n{header}"))
}

/// docs/adr/0086, section 8.6 (c): a host may hand any machine an event by
/// name, and the data of one its document declares no schema for is bounded
/// by the host ABI, `SCE_UNTYPED_EVENT_DATA_MAX_LEN`, 255 bytes. A machine with
/// no script engine holds that data in its slot and checks the bound against
/// the header it compiles with; a machine whose reasons are all shorter has a
/// slot of 255 bytes and a NUL. One with a script engine grows such data in
/// its heap, and relies on no such bound.
#[test]
fn a_static_machine_holds_the_untyped_data_the_host_abi_declares() {
    let doc = machine(
        &[],
        r#"    <transition event="poke"><assign location="seen" expr="1"/></transition>"#,
    );
    let (header, source) = generated(&doc, &[], &[]);
    let reasons = longest_reason(&header);
    assert!(
        reasons < 255,
        "every reason this machine raises is shorter than the untyped bound, not \
         {reasons} bytes:\n{header}"
    );
    assert!(
        header.contains("#define PROBE_DATA_LEN_DECLARED 256u"),
        "with no schema the slot holds the 255 bytes of untyped data and a NUL:\n{header}"
    );
    assert!(
        header.contains("SCE_UNTYPED_EVENT_DATA_MAX_LEN, which the source checks; it\n   is an input too: 255 bytes."),
        "the header names the untyped data as an input:\n{header}"
    );
    assert!(
        header.contains("(docs/adr/0086, section 2.2); it receives none."),
        "the header says it receives no schema's event:\n{header}"
    );
    assert!(
        source.contains(
            "SCE_STATIC_ASSERT((size_t)(SCE_UNTYPED_EVENT_DATA_MAX_LEN) <= (size_t)255u,"
        ),
        "the machine checks the bound its slot was sized by:\n{source}"
    );
    let hosts = [
        "--host-processor",
        "x-sce-host",
        "--host-invoker",
        "x-app-host",
    ];
    let (scripted_header, scripted_source) = generated(SCRIPTED, &[], &hosts);
    for (file, text) in [("header", &scripted_header), ("source", &scripted_source)] {
        assert!(
            !text.contains("SCE_UNTYPED_EVENT_DATA_MAX_LEN"),
            "a script machine's {file} relies on no untyped bound:\n{text}"
        );
    }
}

/// docs/adr/0086, section 8.6 (c): the end of a host-run invocation that
/// declares no result carries data no schema of the document declares, which
/// the host ABI bounds as it bounds an event a host hands by name. A machine
/// with no script engine holds it in its slot, checks the bound, and refuses
/// whole, at its completion door, data past the slot; an invocation that
/// declares its result is held at its schema's largest payload instead
/// (`a_host_run_result_is_an_input_of_the_slot_though_no_transition_names_its_event`).
#[test]
fn an_untyped_host_run_ending_is_held_to_the_bound_the_host_abi_declares() {
    let doc = machine(
        &[],
        r#"    <invoke type="x-app-host" id="ask"/>
    <transition event="done.invoke.ask"><assign location="seen" expr="1"/></transition>
    <transition event="error.invoke.ask"><assign location="seen" expr="2"/></transition>"#,
    );
    let (header, source) = generated(&doc, &[], &["--host-invoker", "x-app-host"]);
    assert!(
        header.contains("as the end of a\n   host-run invocation that declares no result"),
        "the header names an untyped ending as held to the host ABI's bound:\n{header}"
    );
    assert!(
        !header.contains("which a host-run invocation completes with"),
        "an invocation that declares no result is no schema's input:\n{header}"
    );
    assert!(
        source.contains(
            "SCE_STATIC_ASSERT((size_t)(SCE_UNTYPED_EVENT_DATA_MAX_LEN) <= (size_t)255u,"
        ),
        "the machine checks the bound its slot was sized by:\n{source}"
    );
    assert!(
        source.contains("if (data != NULL && strlen(data) >= sizeof(sm->current_external.data)) {\n        refuse_event_data(sm, NULL);\n        return false;\n    }"),
        "an ending whose data is past the slot is refused whole at the door:\n{source}"
    );
    let longest = longest_reason(&header);
    assert!(
        header.contains(&format!(
            "#define PROBE_DATA_LEN_DECLARED {}u",
            longest.max(255) + 1
        )),
        "the slot holds the untyped data, and the host's refusal, with a NUL:\n{header}"
    );
}

/// docs/adr/0086, section 8.6 (b): every reason a C11 machine raises comes
/// from one table, each filled with the document's own text, and the longest
/// is an input of the slot. A list named by 200 bytes makes the reason of an
/// append to it when it is full the longest the machine raises, past every
/// reason of the table such a machine raises that holds no hole, and past the
/// 255 bytes of untyped data the slot holds besides (section 8.6 (c)).
#[test]
fn the_longest_reason_a_machine_raises_is_an_input_of_its_slot() {
    let list = format!("v{}", "x".repeat(199));
    let doc = format!(
        r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="probe">
  <datamodel>
    <data id="{list}" sce:type="list&lt;uint8&gt;[2]" sce:direction="out"/>
    <data id="errors" sce:type="uint32" expr="0" sce:direction="out"/>
  </datamodel>
  <state id="idle">
    <transition event="add" type="internal"><sce:append target="{list}" expr="1"/></transition>
    <transition event="error.execution" type="internal">
      <assign location="errors" expr="errors + 1"/>
    </transition>
  </state>
</scxml>
"#
    );
    let (header, source) = generated(&doc, &[], &[]);
    let full = format!("<sce:append target='{list}'>: the list already holds its capacity of 2");
    // The lowering writes the sentence of a value that fails too, which is
    // longer by one byte; it is measured whether or not the C11 append keeps
    // it, so the longest reason is one of the two.
    let failed = format!("<sce:append target='{list}'>: an integer operation overflowed or failed");
    assert!(
        source.contains(&format!("\"{full}\"")),
        "the append raises the lowering's sentence, through the table:\n{source}"
    );
    let longest = longest_reason(&header);
    assert!(
        longest == full.len() as u64 || longest == failed.len() as u64,
        "the append's reason is the longest the machine raises, {} or {} bytes, not \
         {longest}:\n{header}",
        full.len(),
        failed.len()
    );
    assert!(
        header.contains(&format!("#define PROBE_DATA_LEN_DECLARED {}u", longest + 1)),
        "the slot holds the longest reason and a NUL:\n{header}"
    );
}

// ── docs/adr/0086, section 8.6 (c): the data a machine sends is sized ──────

/// A `sce-static` machine that holds a text of 300 bytes and sends itself an
/// event whose one param, `text`, is the value `param`.
fn sending(param: &str) -> String {
    format!(
        r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="probe">
  <datamodel>
    <data id="seen" sce:type="uint8" expr="0" sce:direction="out"/>
    <data id="label" sce:type="string[300]" expr="'a'" sce:direction="out"/>
  </datamodel>
  <state id="idle">
    <transition event="poke"><send event="told"><param name="text" expr="{param}"/></send></transition>
    <transition event="told"><assign location="seen" expr="1"/></transition>
  </state>
</scxml>
"#
    )
}

/// The data a send writes is an input of the slot, measured by the bound of
/// each value it carries: a typed place at its type's `J`, and a concatenation
/// of the place and a literal at the escapes of the sum of their texts. The
/// payload is `{"text":<value>}`, the braces 2 and the key with its colon 7: a
/// text of 300 bytes is 2 + 6 * 300 = 1802 bytes of JSON, so the payload 1811;
/// `label + '!'` is a text of 301 bytes, 2 + 6 * 301 = 1808, so 1817. Either is
/// past every reason such a machine raises.
#[test]
fn a_send_of_a_typed_place_and_a_literal_grows_the_slot_by_their_sum() {
    let (place, _) = generated(&sending("label"), &[], &[]);
    assert!(
        place.contains("measured by the bound of each value it\n   carries: 1811 bytes."),
        "the place is measured at its type's J:\n{place}"
    );
    assert!(
        place.contains("#define PROBE_DATA_LEN_DECLARED 1812u"),
        "the slot holds the send's 1811 bytes and a NUL:\n{place}"
    );
    let (joined, _) = generated(&sending("label + '!'"), &[], &[]);
    assert!(
        joined.contains("measured by the bound of each value it\n   carries: 1817 bytes."),
        "the join is measured at the sum of its parts' texts:\n{joined}"
    );
    assert!(
        joined.contains("#define PROBE_DATA_LEN_DECLARED 1818u"),
        "the slot grows by the literal's byte, six bytes of JSON at its worst:\n{joined}"
    );
    assert!(
        joined.contains("#define PROBE_DATA_LEN_TO_PARENT 0u"),
        "a machine that sends nothing to a parent hands it nothing:\n{joined}"
    );
}

/// A host-run invocation's deadline is quoted whole by the reason a deadline
/// the machine cannot read is raised with, so its text needs a bound. A
/// literal has one; a value the script engine computes has none, and the
/// machine is refused by name rather than sized by a guess.
#[test]
fn a_c11_deadline_whose_text_has_no_bound_is_refused_by_name() {
    let invoking = |deadline: &str| {
        format!(
            r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="ecmascript" name="probe">
  <datamodel><data id="wait" expr="5000"/></datamodel>
  <state id="s">
    <invoke type="x-app-host" id="ask"><param name="_sce_deadline_ms" expr="{deadline}"/></invoke>
    <transition event="error.execution" target="f"/>
  </state>
  <final id="f"/>
</scxml>
"#
        )
    };
    let flags = ["--host-invoker", "x-app-host"];
    let (_, source) = generated(&invoking("'soon'"), &[], &flags);
    // The type `x-app-host` is 10 bytes, the id `ask` 3, the literal `soon` 4.
    assert!(
        source.contains(
            "\"<invoke type='%.10s' id='%.3s'>: _sce_deadline_ms='%.4s' is not a whole number of \
             milliseconds\""
        ),
        "the reason quotes each text at the bound the build knows of it:\n{source}"
    );
    let (out, _) = run_codegen(&invoking("wait"), &[], &flags);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !out.status.success(),
        "a deadline the script engine computes has no bound to size the reason by:\n{stderr}"
    );
    assert!(
        stderr.contains("<param name=\\\"_sce_deadline_ms\\\" expr=\\\"wait\\\">")
            || stderr.contains("<param name=\"_sce_deadline_ms\" expr=\"wait\">"),
        "the refusal names the param:\n{stderr}"
    );
    assert!(
        stderr.contains("`wait`") && stderr.contains("8.6 (c)"),
        "the refusal names the expression that has no bound, and why:\n{stderr}"
    );
}

/// A child session publishes what it hands its parent, its sends to
/// `#_parent`, and the parent's slot holds the largest each child publishes.
/// The child sends `{"greeting":"hi"}`, 17 bytes, every pair a literal.
#[test]
fn a_parents_slot_holds_what_each_child_hands_it() {
    let doc = r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="ecmascript" name="probe">
  <state id="s">
    <invoke type="scxml" id="kid">
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="k"
               datamodel="ecmascript" name="kid">
          <state id="k">
            <onentry><send target="#_parent" event="hello"><param name="greeting" expr="'hi'"/></send></onentry>
          </state>
        </scxml>
      </content>
    </invoke>
    <transition event="hello" target="f"/>
  </state>
  <final id="f"/>
</scxml>
"##;
    let (out, out_dir) = run_codegen(doc, &[], &[]);
    assert!(
        out.status.success(),
        "the machine generates for C11:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let parent = std::fs::read_to_string(out_dir.path().join("probe_sm.h")).expect("the parent");
    let child_stem = parent
        .lines()
        .filter_map(|line| line.strip_prefix("#include \""))
        .filter_map(|rest| rest.strip_suffix("_sm.h\""))
        .find(|stem| *stem != "probe")
        .unwrap_or_else(|| panic!("the parent includes its child's header:\n{parent}"))
        .to_string();
    let child = std::fs::read_to_string(out_dir.path().join(format!("{child_stem}_sm.h")))
        .expect("the child's header, written by the run that writes its parent");
    let published = format!("{}_DATA_LEN_TO_PARENT", child_stem.to_uppercase());
    assert!(
        child.contains(&format!("#define {published} 17u")),
        "the child publishes the 17 bytes it sends its parent:\n{child}"
    );
    assert!(
        parent.contains(&format!(
            "#define PROBE_DATA_LEN PROBE_DATA_LEN_LARGER(PROBE_DATA_LEN_OWN, {published})"
        )),
        "the parent's slot holds what its child publishes:\n{parent}"
    );
}

/// docs/adr/0086, section 8.6 (c): a child with no script engine under a data
/// model other than `sce-static` holds its done data, an inline `<content>`
/// finished at build time, in a buffer of its own slot, and hands it to its
/// parent on `done.invoke`. The done data is measured as the text it is
/// finished as: under the Null data model 300 bytes of text are the JSON
/// string of them, 302 bytes. The child's slot grows to hold it, the child
/// publishes it for its parent, and the parent's slot holds what it publishes.
/// A child whose data model is a script engine makes its parent run one too,
/// so no machine with a slot is handed done data a script engine computed.
#[test]
fn a_childs_done_data_grows_the_slots_that_hold_it() {
    let text = "a".repeat(300);
    let doc = format!(
        r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" initial="s" datamodel="null" name="probe">
  <state id="s">
    <invoke type="scxml" id="kid">
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="k"
               datamodel="null" name="kid">
          <final id="k"><donedata><content>{text}</content></donedata></final>
        </scxml>
      </content>
    </invoke>
    <transition event="done.invoke.kid" target="f"/>
  </state>
  <final id="f"/>
</scxml>
"##
    );
    let (out, out_dir) = run_codegen(&doc, &[], &[]);
    assert!(
        out.status.success(),
        "the machine generates for C11:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let parent = std::fs::read_to_string(out_dir.path().join("probe_sm.h")).expect("the parent");
    let child_stem = parent
        .lines()
        .filter_map(|line| line.strip_prefix("#include \""))
        .filter_map(|rest| rest.strip_suffix("_sm.h\""))
        .find(|stem| *stem != "probe")
        .unwrap_or_else(|| panic!("the parent includes its child's header:\n{parent}"))
        .to_string();
    let child = std::fs::read_to_string(out_dir.path().join(format!("{child_stem}_sm.h")))
        .expect("the child's header, written by the run that writes its parent");
    let slot = format!("{}_DATA_LEN", child_stem.to_uppercase());
    assert!(
        child.contains("measured by the bound of each value it\n   carries: 302 bytes."),
        "the child's done data is an input of its own slot:\n{child}"
    );
    assert!(
        child.contains(&format!("#define {slot}_DECLARED 303u")),
        "the child's slot holds its 302 bytes of done data and a NUL:\n{child}"
    );
    assert!(
        child.contains(&format!("#define {slot}_TO_PARENT 302u")),
        "the child publishes the done data it hands its parent:\n{child}"
    );
    assert!(
        parent.contains(&format!(
            "#define PROBE_DATA_LEN PROBE_DATA_LEN_LARGER(PROBE_DATA_LEN_OWN, {slot}_TO_PARENT)"
        )),
        "the parent's slot holds what its child publishes:\n{parent}"
    );
}

/// docs/adr/0086, section 8.5 (c): each queue of a C11 machine holds as many
/// events as the document's `sce:capacity` declares, so a machine that takes
/// large events reserves room for few of them; a document that declares none
/// takes the deployment's SCE_MAX_EVENTS.
#[test]
fn a_c11_queue_takes_the_depth_its_document_declares() {
    let body = r#"    <transition event="poke"><assign location="seen" expr="1"/></transition>"#;
    let undeclared = header(&machine(&[], body), &[], None);
    assert!(
        undeclared.contains("#define PROBE_EVENT_QUEUE_DEPTH ((unsigned)(SCE_MAX_EVENTS))"),
        "a document that declares no depth takes the deployment's:\n{undeclared}"
    );
    let declared_doc =
        machine(&[], body).replace(r#"name="probe">"#, r#"name="probe" sce:capacity="4">"#);
    assert!(
        declared_doc.contains(r#"sce:capacity="4""#),
        "the probe declares its depth"
    );
    let declared = header(&declared_doc, &[], None);
    assert!(
        declared.contains("#define PROBE_EVENT_QUEUE_DEPTH 4u"),
        "the depth is the one the document declares:\n{declared}"
    );
    assert!(
        record(&declared, "probe_event_queue")
            .contains("probe_queued_event_t buf[PROBE_EVENT_QUEUE_DEPTH];"),
        "each queue holds that many entries:\n{declared}"
    );
}

/// docs/adr/0086, section 8.5 (c): a queue's depth is counted in 16 bits, so a
/// document that declares more than 255 events puts each one at its own index,
/// the ones past 255 included, and the ring's head and tail wrap at the depth
/// and never at 256. Compiled and run; skipped when no C compiler is present.
#[test]
fn a_c11_queue_deeper_than_255_holds_each_event_at_its_own_index() {
    let Some(cc) = sce_build::toolchain::locate_any(&["gcc", "clang"]) else {
        sce_build::toolchain::skipped(
            "a_c11_queue_deeper_than_255_holds_each_event_at_its_own_index: gcc/clang not on PATH",
        );
        return;
    };
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("probe.scxml");
    std::fs::write(
        &path,
        r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="waiting" datamodel="ecmascript" name="probe" sce:capacity="300">
  <state id="waiting">
    <transition event="poke"/>
  </state>
</scxml>
"#,
    )
    .expect("write the machine");
    let generated = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("generate")
        .arg(&path)
        .args(["-l", "c11", "-o"])
        .arg(dir.path())
        .output()
        .expect("invoke sce-codegen");
    assert!(
        generated.status.success(),
        "the machine generates for C11:\n{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let driver = dir.path().join("driver.c");
    std::fs::write(
        &driver,
        r#"#include "probe_sm.h"
#include <stdio.h>
#include <string.h>
_Static_assert(PROBE_EVENT_QUEUE_DEPTH == 300u, "the depth is the one the document declares");
static probe_t sm;
static int fail(const char *what) {
    (void)fprintf(stderr, "FAIL: %s\n", what);
    return 1;
}
int main(void) {
    char want[SCE_MAX_ID_LEN];
    probe_init(&sm);
    probe_run(&sm);
    /* One event more than the queue holds: the last is dropped. */
    for (unsigned i = 0u; i <= 300u; ++i) {
        probe_event_with_meta_t meta;
        memset(&meta, 0, sizeof(meta));
        meta.event = PROBE_EVENT_POKE;
        (void)snprintf(meta.send_id, sizeof(meta.send_id), "%u", i);
        probe_raise_external(&sm, &meta);
    }
    if (sm.external_queue.count != 300u) {
        return fail("the queue does not hold the 300 events its document declares");
    }
    if (sm.external_queue.tail != 0u) {
        return fail("the tail did not wrap at the depth");
    }
    for (unsigned i = 0u; i < 300u; ++i) {
        (void)snprintf(want, sizeof(want), "%u", i);
        if (strcmp(sm.external_queue.buf[i].send_id, want) != 0) {
            (void)fprintf(stderr, "FAIL: index %u holds the event sent as `%s`\n", i,
                          sm.external_queue.buf[i].send_id);
            return 1;
        }
    }
    probe_run(&sm);
    if (sm.external_queue.count != 0u || sm.external_queue.head != 0u) {
        return fail("the run did not take every event, its head wrapping at the depth");
    }
    return 0;
}
"#,
    )
    .expect("write the driver");
    let bin = dir.path().join("driver");
    let built = Command::new(&cc)
        .args([
            "-std=c11",
            "-O0",
            "-Wall",
            "-Wextra",
            "-Wpedantic",
            "-Werror",
        ])
        .arg("-I")
        .arg(repo_root().join("backends/c/runtime/include"))
        .arg("-I")
        .arg(dir.path())
        .arg(dir.path().join("probe_sm.c"))
        .arg(&driver)
        .arg("-o")
        .arg(&bin)
        .output()
        .expect("run the C compiler");
    assert!(
        built.status.success(),
        "the machine and its driver compile:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let ran = Command::new(&bin).output().expect("run the driver");
    assert!(
        ran.status.success(),
        "a queue deeper than 255 holds each event at its own index:\n{}",
        String::from_utf8_lossy(&ran.stderr)
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

/// docs/adr/0086, section 8.5 (e): the HTTP host machine, whose slot holds the
/// HTTP result's 123074 bytes, declares its scratch as one union per colour of
/// its roles' interference graph, each role inside one of them under its own
/// name, so the scratch costs the sum over colours and not over roles.
#[test]
fn the_http_host_machines_scratch_is_one_union_per_colour_not_one_member_per_role() {
    let doc = std::fs::read_to_string(
        repo_root().join("sce-build/tests/fixtures/static_datamodel/static_host_http.scxml"),
    )
    .expect("the HTTP host machine");
    // A machine's C symbols are named after its file's stem, never its `name`
    // attribute, and [`generated`] writes every document as `probe.scxml`.
    let (header, _) = generated(&doc, &[], &[]);
    let start = header
        .find("typedef struct probe_scratch {")
        .expect("the machine's scratch record");
    let end = start
        + header[start..]
            .find("} probe_scratch_t;")
            .expect("the end of the scratch record");
    let record = &header[start..end];
    let unions = record.matches("    union {").count();
    // The replies of a host-served send are held by a machine that serves one,
    // which the HTTP host machine, whose work is a host-run invocation, is not.
    // The machine is a `sce-static` one, with no script engine.
    let holder = sce_build::forge::c11_scratch::Holder {
        host_sends: record.contains("char replies["),
        script_engine: false,
    };
    let layout = sce_build::forge::c11_scratch::layout(holder);
    let roles: Vec<&sce_build::forge::c11_scratch::Role> = sce_build::forge::c11_scratch::ROLES
        .iter()
        .filter(|role| sce_build::forge::c11_scratch::held(role, holder))
        .collect();
    assert_eq!(unions, layout.len(), "one union per colour:\n{record}");
    assert!(
        layout.len() < roles.len(),
        "the roles share storage: {} colours for {} roles",
        layout.len(),
        roles.len()
    );
    for role in &roles {
        let declared = record.matches(&format!("char {}[", role.name)).count();
        assert_eq!(declared, 1, "`{}` is declared once:\n{record}", role.name);
    }
    let members = record.matches("        char ").count();
    assert_eq!(
        members,
        roles.len(),
        "every member of the record is a role inside a union:\n{record}"
    );
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
      <send event="w" type="x-sce-host" delay="1s"><param name="p" expr="'y' + v"/></send>
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
/// guard reads. The invocation carries a deadline and the document answers
/// error.execution, so its start writes the reason an unreadable deadline is
/// raised with (the `reason` activity).
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
    <invoke type="x-app-host" id="ask" sce:result="Answer">
      <param name="n" expr="count"/>
      <param name="_sce_deadline_ms" expr="1000"/>
    </invoke>
    <transition event="job.done" cond="_event.data.label === 'ok'"><assign location="count" expr="1"/></transition>
    <transition event="error.execution"><assign location="count" expr="2"/></transition>
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

// ── docs/adr/0086, section 8.5 (a): a script machine owns each event's text ──

/// The text of the record `typedef struct <name> { ... } <name>_t;` of `header`.
fn record<'a>(header: &'a str, name: &str) -> &'a str {
    let start = header
        .find(&format!("typedef struct {name} {{"))
        .unwrap_or_else(|| panic!("the record `{name}`:\n{header}"));
    let end = start
        + header[start..]
            .find(&format!("}} {name}_t;"))
            .unwrap_or_else(|| panic!("the end of `{name}`:\n{header}"));
    &header[start..end]
}

/// A machine whose data model is a script engine holds each event's data in a
/// text its engine allocates, in its queued events and its scheduled entries,
/// and no slot of `_event.data` there; a post to its inbox hands over storage
/// with the function that releases it; and it frees every text it holds before
/// it closes the engine whose allocator gave them. It has no saved state, so
/// no text crosses a save.
#[test]
fn a_script_machine_holds_each_event_text_it_owns_and_frees_them_before_its_engine() {
    let (header, source) = generated(
        SCRIPTED,
        &[],
        &[
            "--host-processor",
            "x-sce-host",
            "--host-invoker",
            "x-app-host",
        ],
    );
    let queued = record(&header, "probe_queued_event");
    assert!(
        queued.contains("    char *data;\n") && queued.contains("    size_t data_size;\n"),
        "a queued event owns a text and knows its size:\n{queued}"
    );
    assert!(
        !queued.contains("_DATA_LEN]"),
        "a queued event holds no slot:\n{queued}"
    );
    let scheduled = record(&header, "probe_scheduled_entry");
    assert!(
        scheduled.contains("    char *data;\n") && !scheduled.contains("char data["),
        "a scheduled entry owns a text and holds no slot for it:\n{scheduled}"
    );
    assert!(
        scheduled.contains("    char *host_content;\n")
            && scheduled.contains("    char *host_event_data;\n")
            && scheduled.contains("    char *host_param_values[1];\n")
            && !scheduled.contains("_DATA_LEN]"),
        "a delayed host-served send owns its texts and holds no slot for them:\n{scheduled}"
    );
    let input = record(&header, "probe_inbox_input");
    // docs/adr/0086, section 8.6 (d): a host-run invocation's ending too.
    assert_eq!(
        input.matches("sce_inbox_release_fn release;").count(),
        3,
        "an event, an event by name and an ending each hand over the poster's storage:\n{input}"
    );
    assert!(
        !input.contains("_DATA_LEN]"),
        "no input of the inbox holds a slot:\n{input}"
    );
    assert!(
        header.contains("bool probe_inbox_post_ending(probe_t *sm,")
            && header.contains("sce_inbox_release_fn release, void *release_context);"),
        "a post of an ending hands over its data with the function that releases it:\n{header}"
    );
    assert!(
        !header.contains("bool probe_save("),
        "a script machine has no saved state:\n{header}"
    );
    assert!(
        header.contains("uint32_t probe_owned_event_texts(const probe_t *sm);"),
        "the machine says how many texts it holds:\n{header}"
    );
    let destroy = source
        .find("SCE_SM_FN void probe_destroy(probe_t *sm) {")
        .expect("the machine's destroy");
    let body = &source[destroy..];
    let freed = body
        .find("release_owned_event_texts(sm);")
        .expect("destroy frees the texts the machine holds");
    let closed = body
        .find("lua_close(sm->L);")
        .expect("destroy closes the engine");
    assert!(
        freed < closed,
        "every text is freed before the engine whose allocator gave it is closed"
    );
}

/// docs/adr/0086, section 8.6 (d): a machine whose data model is a script
/// engine grows a host's replies and a host-run invocation's answer in its
/// engine's heap, through the runtime's writers, and frees them once raised:
/// its scratch holds neither role, its calls hand the handler the engine's
/// allocator, and each answer is released after it is raised. A typed machine
/// keeps both in its scratch.
#[test]
fn a_script_machine_grows_host_replies_and_answers_in_its_heap() {
    let hosts = [
        "--host-processor",
        "x-sce-host",
        "--host-invoker",
        "x-app-host",
    ];
    let (header, source) = generated(SCRIPTED, &[], &hosts);
    let scratch = record(&header, "probe_scratch");
    assert!(
        !scratch.contains("char replies[") && !scratch.contains("char answer["),
        "a script machine's scratch holds no reply or answer:\n{scratch}"
    );
    assert!(
        source.contains("_replies.alloc = lua_getallocf(sm->L, &_replies.alloc_context);")
            && source.contains("out.alloc = lua_getallocf(sm->L, &out.alloc_context);"),
        "the replies and the answer grow through the engine's allocator:\n{source}"
    );
    assert!(
        !source.contains("sm->scratch.replies") && !source.contains("sm->scratch.answer"),
        "no reply or answer is written into the scratch:\n{source}"
    );
    assert_eq!(
        source
            .matches("sce_host_invoke_response_release(&out);")
            .count(),
        2,
        "the answer is released after a refusal and after a completion:\n{source}"
    );
    assert!(
        source.contains("sce_host_response_list_release(&_replies);"),
        "the replies are released once raised:\n{source}"
    );
    let (typed, _) = generated(
        &typed_machine(),
        &[
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
        ],
        &hosts,
    );
    let typed_scratch = record(&typed, "probe_scratch");
    assert!(
        typed_scratch.contains("char replies[") && typed_scratch.contains("char answer["),
        "a typed machine keeps both in its scratch:\n{typed_scratch}"
    );
}

/// A machine without a script engine keeps its event data in its slot, as
/// section 8.1 sizes it, and hands nothing over through its inbox.
#[test]
fn a_typed_machine_keeps_its_event_data_in_its_slot() {
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
    let (header, _) = generated(
        &typed_machine(),
        &typed_schemas,
        &[
            "--host-processor",
            "x-sce-host",
            "--host-invoker",
            "x-app-host",
        ],
    );
    let queued = record(&header, "probe_queued_event");
    assert!(
        queued.contains("    char data[PROBE_DATA_LEN];\n") && !queued.contains("char *data;"),
        "a typed machine's queued event holds the slot:\n{queued}"
    );
    assert!(
        !record(&header, "probe_inbox_input").contains("sce_inbox_release_fn"),
        "a typed machine's inbox copies each post into its own slot"
    );
    assert!(
        !header.contains("probe_owned_event_texts"),
        "a typed machine allocates no text:\n{header}"
    );
}

/// docs/adr/0086, section 8.6 (d): a machine whose data model is a script
/// engine holds each role its engine's values are written into as a buffer of
/// its own that grows in the engine's heap, reaches it only through the
/// scratch's accessors, and frees it before the engine is closed: no site of
/// the generated code sizes such a role by a slot (`sizeof(sm->scratch.<role>`
/// would be a pointer's size once the role is a buffer), and no text is copied
/// into one cut. A typed machine keeps every role in fixed storage, reached
/// through the same accessors spelled over it.
#[test]
fn a_script_machine_grows_its_scratch_and_sizes_no_grown_role_by_a_slot() {
    let hosts = [
        "--host-processor",
        "x-sce-host",
        "--host-invoker",
        "x-app-host",
    ];
    let scripted = generated(SCRIPTED, &[], &hosts);
    let children = generated(CHILDREN, &[], &hosts);
    for (header, source) in [&scripted, &children] {
        let scratch = record(header, "probe_scratch");
        for role in c11_scratch::GROWN_IN_THE_SCRATCH {
            for (file, text) in [("header", header), ("source", source)] {
                assert!(
                    !text.contains(&format!("sizeof(sm->scratch.{role}")),
                    "the {file} sizes the grown role `{role}` by a slot:\n{text}"
                );
            }
            assert!(
                scratch.contains(&format!("    sce_grown_text_t {role}")),
                "`{role}` is a buffer of its own that grows:\n{scratch}"
            );
        }
        assert!(
            source.contains(
                "#define SCE_SCRATCH_RESERVE(role, need) scratch_reserve(sm, &sm->scratch.role, \
                 (size_t)(need))"
            ),
            "a reserve grows the buffer through the engine's allocator:\n{source}"
        );
        assert!(
            source.contains("lua_getallocf(sm->L, &alloc_context);"),
            "the scratch grows in the engine's own heap:\n{source}"
        );
        let release = source
            .find("SCE_SM_FN static void release_owned_event_texts(")
            .expect("the machine frees what it holds");
        let release_end = release
            + source[release..]
                .find("\n}\n")
                .expect("the end of the release");
        assert!(
            source[release..release_end].contains("release_scratch(sm);"),
            "the scratch is freed with every text, before the engine is closed:\n{source}"
        );
        assert!(
            !source.contains("sce_copy_bounded_n(_host_param_values"),
            "no pair's text is cut into a row:\n{source}"
        );
    }
    assert!(
        scripted
            .1
            .contains("if (!SCE_SCRATCH_RESERVE(payload, _jl + 1u)) {"),
        "a send's payload grows to the text its engine computed:\n{}",
        scripted.1
    );
    assert!(
        !record(&scripted.0, "probe_scratch").contains("char payload["),
        "a script machine's payload is no slot"
    );
    assert!(
        children.1.contains(", NULL, 0u);")
            && children
                .1
                .contains("if (SCE_SCRATCH_RESERVE(done, _done_whole + 1u)) {"),
        "a child's done data is measured, and the scratch grown to it:\n{}",
        children.1
    );
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
    let (typed_header, typed_source) = generated(&typed_machine(), &typed_schemas, &hosts);
    assert!(
        !record(&typed_header, "probe_scratch").contains("sce_grown_text_t"),
        "a typed machine grows nothing"
    );
    assert!(
        typed_source.contains(
            "#define SCE_SCRATCH_RESERVE(role, need) ((size_t)(need) <= sizeof(sm->scratch.role))"
        ),
        "a typed machine's reserve asks whether the text fits its slot:\n{typed_source}"
    );
}

// ── docs/adr/0086, section 8.5 (e): the containment table, checked ──────────

use sce_build::forge::c11_scratch::{self, Activity};

/// A function the generated C defines: its name, whether a host may call it
/// (it is not `static`), and its body as character offsets of the code.
struct CFunction {
    name: String,
    public: bool,
    body: (usize, usize),
}

/// Whether `c` may stand in a C identifier.
fn ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Every function the code-only text `code` defines: a body is what a `{`
/// opens at the top level after a `)`, the name the identifier before that
/// `)`'s matching `(`, and the function public when the text since the last
/// top-level `;` or `}` does not say `static`.
fn c_functions(code: &[char]) -> Vec<CFunction> {
    let mut found = Vec::new();
    let mut depth = 0usize;
    let mut signature_start = 0usize;
    let mut last_significant: Option<usize> = None;
    let mut open_at = 0usize;
    let mut pending: Option<(String, bool)> = None;
    for (at, &ch) in code.iter().enumerate() {
        match ch {
            '{' => {
                if depth == 0 {
                    pending = None;
                    if let Some(close) = last_significant.filter(|&i| code[i] == ')') {
                        let mut parens = 0i32;
                        let mut open = close;
                        loop {
                            match code[open] {
                                ')' => parens += 1,
                                '(' => parens -= 1,
                                _ => {}
                            }
                            if parens == 0 || open == 0 {
                                break;
                            }
                            open -= 1;
                        }
                        let mut end = open;
                        while end > 0 && code[end - 1].is_whitespace() {
                            end -= 1;
                        }
                        let mut start = end;
                        while start > 0 && ident_char(code[start - 1]) {
                            start -= 1;
                        }
                        let name: String = code[start..end].iter().collect();
                        let signature: String = code[signature_start..at].iter().collect();
                        if !name.is_empty() {
                            pending = Some((name, !signature.contains("static")));
                            open_at = at;
                        }
                    }
                }
                depth += 1;
            }
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    if let Some((name, public)) = pending.take() {
                        found.push(CFunction {
                            name,
                            public,
                            body: (open_at, at),
                        });
                    }
                    signature_start = at + 1;
                }
            }
            ';' if depth == 0 => signature_start = at + 1,
            _ => {}
        }
        if !ch.is_whitespace() {
            last_significant = Some(at);
        }
    }
    found
}

/// A call the code makes: the name it calls, or a call through a member of a
/// struct (`x->f(`, `x.f(`), a function pointer, by the member's name.
enum CCall {
    Named(String),
    Pointer(String),
}

/// Every call in `code[from..to]`.
fn c_calls(code: &[char], from: usize, to: usize) -> Vec<CCall> {
    const KEYWORDS: [&str; 6] = ["if", "while", "for", "switch", "sizeof", "return"];
    let mut calls = Vec::new();
    let mut at = from;
    while at < to {
        if ident_char(code[at]) && (at == 0 || !ident_char(code[at - 1])) {
            let start = at;
            while at < to && ident_char(code[at]) {
                at += 1;
            }
            let name: String = code[start..at].iter().collect();
            let mut after = at;
            while after < to && code[after].is_whitespace() {
                after += 1;
            }
            if after < to && code[after] == '(' && !KEYWORDS.contains(&name.as_str()) {
                let member = (start >= 1 && code[start - 1] == '.')
                    || (start >= 2 && code[start - 2] == '-' && code[start - 1] == '>');
                calls.push(if member {
                    CCall::Pointer(name)
                } else {
                    CCall::Named(name)
                });
            }
        } else {
            at += 1;
        }
    }
    calls
}

/// Each activity the raw text `raw` marks (`/* sce-scratch-activity: <name> */`)
/// with the span it runs over in the code: from its mark to the end of the
/// block that holds the mark.
fn c_marks(raw: &[char], code: &[char]) -> Vec<(Activity, usize, usize)> {
    const MARK: &str = "/* sce-scratch-activity: ";
    let text: String = raw.iter().collect();
    let mut marks = Vec::new();
    for (byte_at, _) in text.match_indices(MARK) {
        let at = text[..byte_at].chars().count();
        let name_start = at + MARK.chars().count();
        let name: String = raw[name_start..]
            .iter()
            .take_while(|c| ident_char(**c))
            .collect();
        let activity = Activity::marked(&name)
            .unwrap_or_else(|| panic!("`{name}` marks no activity of c11_scratch"));
        let mut depth = 0i64;
        let mut end = at;
        while end < code.len() {
            match code[end] {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth < 0 {
                        break;
                    }
                }
                _ => {}
            }
            end += 1;
        }
        marks.push((activity, at, end));
    }
    marks
}

/// The functions `from` reaches through the calls `callees` records, itself
/// included, each by the name `by_name` holds it under.
fn reached<'a>(
    from: &str,
    by_name: &std::collections::BTreeMap<&'a str, &CFunction>,
    callees: &std::collections::BTreeMap<&'a str, std::collections::BTreeSet<&'a str>>,
) -> std::collections::BTreeSet<&'a str> {
    let mut seen: std::collections::BTreeSet<&'a str> = std::collections::BTreeSet::new();
    let mut pending: Vec<&'a str> = match by_name.get_key_value(from) {
        Some((name, _)) => vec![*name],
        None => Vec::new(),
    };
    while let Some(at) = pending.pop() {
        if seen.insert(at) {
            if let Some(next) = callees.get(at) {
                pending.extend(next.iter().copied());
            }
        }
    }
    seen
}

/// A call through a member that runs this machine's host code, which is a host
/// callback; `parent_dispatch` runs the parent machine, whose scratch is its
/// own, and is not one, and neither is `release`, the deleter a poster hands
/// its storage over with, which by its contract (`sce_inbox_release_fn`,
/// sce/inbox.h) calls no function of the machine.
fn is_host_callback(member: &str) -> bool {
    member != "parent_dispatch" && member != "release"
}

/// The containments the generated C of one machine (`header` and `source`)
/// shows, as `(outer, inner)`: inside the span of each marked activity, each
/// activity a call there can enter, at any depth of the calls the machine's
/// own functions make; and inside a host callback, each activity a function
/// the host may call from a handler can enter, every public function that
/// does not drive the machine (run a macrostep, start invocations, restore).
fn observed_containments(
    header: &str,
    source: &str,
) -> std::collections::BTreeSet<(Activity, Activity)> {
    use std::collections::{BTreeMap, BTreeSet};
    let raw: Vec<char> = format!("{header}\n{source}").chars().collect();
    let code: Vec<char> = code_only(&raw.iter().collect::<String>()).chars().collect();
    let functions = c_functions(&code);
    let marks = c_marks(&raw, &code);
    let by_name: BTreeMap<&str, &CFunction> =
        functions.iter().map(|f| (f.name.as_str(), f)).collect();
    // What each function calls by name, what it marks, and whether it calls
    // host code.
    let mut callees: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    let mut marked: BTreeMap<&str, BTreeSet<Activity>> = BTreeMap::new();
    let mut calls_host: BTreeSet<&str> = BTreeSet::new();
    for f in &functions {
        for call in c_calls(&code, f.body.0, f.body.1) {
            match call {
                CCall::Named(name) => {
                    if let Some((callee, _)) = by_name.get_key_value(name.as_str()) {
                        callees.entry(f.name.as_str()).or_default().insert(*callee);
                    }
                }
                CCall::Pointer(member) => {
                    if is_host_callback(&member) {
                        calls_host.insert(f.name.as_str());
                    }
                }
            }
        }
        for (activity, at, _) in &marks {
            if *at > f.body.0 && *at < f.body.1 {
                marked.entry(f.name.as_str()).or_default().insert(*activity);
            }
        }
    }
    let reach = |from: &str| reached(from, &by_name, &callees);
    // A function that drives the machine: one that runs executable content,
    // a macrostep, the start of invocations or a restore, which a host does
    // not call from inside a handler (docs/adr/0084: its owner drives it at a
    // macrostep boundary).
    let drives = |name: &str| {
        reach(name).iter().any(|f| {
            f.contains("execute_")
                || f.ends_with("microstep")
                || f.ends_with("run_main_event_loop")
                || f.ends_with("_saved_events")
        })
    };
    let host_api: Vec<&str> = functions
        .iter()
        .filter(|f| f.public && !drives(&f.name))
        .map(|f| f.name.as_str())
        .collect();
    // What a host callback can enter: what the functions a host may call can.
    let mut callback_enters: BTreeSet<Activity> = BTreeSet::new();
    for name in &host_api {
        for f in reach(name) {
            callback_enters.extend(marked.get(f).into_iter().flatten().copied());
        }
    }
    // What a call of `name` can enter.
    let enters = |name: &str| -> BTreeSet<Activity> {
        let mut entered = BTreeSet::new();
        for f in reach(name) {
            entered.extend(marked.get(f).into_iter().flatten().copied());
            if calls_host.contains(f) {
                entered.insert(Activity::HostCallback);
                entered.extend(callback_enters.iter().copied());
            }
        }
        entered
    };
    let mut observed = BTreeSet::new();
    for inner in &callback_enters {
        if *inner != Activity::HostCallback {
            observed.insert((Activity::HostCallback, *inner));
        }
    }
    for (outer, from, to) in &marks {
        let mut inside: BTreeSet<Activity> = BTreeSet::new();
        for call in c_calls(&code, *from, *to) {
            match call {
                CCall::Named(name) => inside.extend(enters(&name)),
                CCall::Pointer(member) => {
                    if is_host_callback(&member) {
                        inside.insert(Activity::HostCallback);
                        inside.extend(callback_enters.iter().copied());
                    }
                }
            }
        }
        for (nested, at, _) in &marks {
            if at > from && at < to {
                inside.insert(*nested);
            }
        }
        for inner in inside {
            if inner != *outer {
                observed.insert((*outer, inner));
            }
        }
    }
    observed
}

/// A machine with a script engine whose child session sends to `#_parent`
/// and ends with done data, which makes a BasicHTTP request, and which raises
/// a compound final's done data from its engine.
const CHILDREN: &str = r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="ecmascript" name="probe">
  <datamodel><data id="v" expr="1"/></datamodel>
  <state id="s">
    <onentry>
      <send event="p" type="http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor"
            target="http://localhost:1/probe"><param name="a" expr="v"/></send>
    </onentry>
    <invoke type="scxml" id="kid">
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="k"
               datamodel="ecmascript" name="kid">
          <state id="k">
            <onentry><send target="#_parent" event="hello"/></onentry>
            <transition target="kf"/>
          </state>
          <final id="kf"><donedata><param name="x" expr="1"/></donedata></final>
        </scxml>
      </content>
    </invoke>
    <transition event="hello" target="t"/>
  </state>
  <state id="t">
    <state id="t1"><transition event="fin" target="tf"/></state>
    <final id="tf"><donedata><param name="k" expr="v"/></donedata></final>
  </state>
</scxml>
"##;

/// Every containment the generated C of a corpus that runs every activity
/// shows is one the table of containments states, through its transitive
/// closure: the table the colouring rests on is checked against the code, not
/// trusted (docs/adr/0086, section 8.5 (e)).
#[test]
fn every_containment_the_generated_code_shows_is_in_the_table() {
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
    let mut seen_marks: std::collections::BTreeSet<Activity> = std::collections::BTreeSet::new();
    let mut unstated = Vec::new();
    for (kind, doc, schemas) in [
        ("a script engine's", SCRIPTED.to_string(), &[][..]),
        ("a typed", typed_machine(), &typed_schemas[..]),
        ("a parent's", CHILDREN.to_string(), &[][..]),
    ] {
        let (header, source) = generated(&doc, schemas, &hosts);
        let raw: Vec<char> = format!("{header}\n{source}").chars().collect();
        let code: Vec<char> = code_only(&raw.iter().collect::<String>()).chars().collect();
        seen_marks.extend(c_marks(&raw, &code).into_iter().map(|(a, _, _)| a));
        for (outer, inner) in observed_containments(&header, &source) {
            if !c11_scratch::contains(outer, inner) {
                unstated.push(format!("{kind} machine: {outer:?} contains {inner:?}"));
            }
        }
    }
    assert!(
        unstated.is_empty(),
        "the generated code shows containments the table does not state:\n{}",
        unstated.join("\n")
    );
    // The corpus runs the activities whose containments the table states, so a
    // missing edge among them is seen.
    for activity in [
        Activity::Send,
        Activity::InvokeStart,
        Activity::HostReply,
        Activity::HostAnswer,
        Activity::Drain,
        Activity::Seam,
        Activity::Result,
        Activity::Reply,
        Activity::Lift,
        Activity::Reason,
    ] {
        assert!(
            seen_marks.contains(&activity),
            "the corpus runs no {activity:?}, so its containments go unchecked"
        );
    }
}

/// The parse of the generated text finds functions, the calls in a span and
/// the marks with their blocks, as the containment test reads them.
#[test]
fn the_parse_reads_functions_calls_and_marked_blocks() {
    let c = "static int helper(int x) { return x; }\n\
             int probe_api(void) {\n    \
             /* sce-scratch-activity: seam */\n    \
             helper(1);\n    \
             sm->clock.now();\n\
             }\n";
    let raw: Vec<char> = c.chars().collect();
    let code: Vec<char> = code_only(c).chars().collect();
    let functions = c_functions(&code);
    let names: Vec<(&str, bool)> = functions
        .iter()
        .map(|f| (f.name.as_str(), f.public))
        .collect();
    assert_eq!(names, vec![("helper", false), ("probe_api", true)]);
    let marks = c_marks(&raw, &code);
    assert_eq!(marks.len(), 1);
    assert_eq!(marks[0].0, Activity::Seam);
    let calls: Vec<String> = c_calls(&code, marks[0].1, marks[0].2)
        .into_iter()
        .map(|call| match call {
            CCall::Named(name) => format!("named {name}"),
            CCall::Pointer(name) => format!("pointer {name}"),
        })
        .collect();
    assert_eq!(calls, vec!["named helper", "pointer now"]);
}

/// A machine with a script engine whose only sends are BasicHTTP ones: a
/// namelist (W3C test518), a `<param>` its engine computes, and a literal
/// `<param>` (W3C test519).
const BASIC_HTTP_SENDS: &str = r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s"
       datamodel="ecmascript" name="probe">
  <datamodel><data id="v" expr="1"/></datamodel>
  <state id="s">
    <onentry>
      <send event="named" type="http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor"
            target="http://localhost:1/probe" namelist="v"/>
      <send event="computed" type="http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor"
            target="http://localhost:1/probe"><param name="a" expr="v + 1"/></send>
      <send event="written" type="http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor"
            target="http://localhost:1/probe"><param name="b" expr="'x'"/></send>
    </onentry>
  </state>
</scxml>
"##;

/// The BasicHTTP processor's request carries its pairs as the POST's form and
/// no payload, so its send evaluates the pairs once and writes no JSON of them
/// into the scratch's payload: no `_ed` is declared, which the compiler would
/// refuse as set and never read, and the request's event data is empty. A send
/// a host serves is handed the payload, and still builds it.
#[test]
fn a_basic_http_send_builds_no_payload_and_a_host_served_send_does() {
    let (_, source) = generated(BASIC_HTTP_SENDS, &[], &[]);
    assert!(
        !source.contains("const char *_ed"),
        "a BasicHTTP send declares no payload:\n{source}"
    );
    assert!(
        !source.contains("JSON.stringify(_scxml_params("),
        "a BasicHTTP send serialises no payload:\n{source}"
    );
    assert!(
        source.contains("(void)luaL_dostring(sm->L, \"_sce_send_pairs = {}\");")
            && source.contains("o[#o + 1] = p[1]; o[#o + 1] = _w(p[2])"),
        "the namelist and the computed param are evaluated once and read as text:\n{source}"
    );
    assert!(
        source.contains("static const char _host_param_value_0[] = "),
        "the literal param crosses as its literal:\n{source}"
    );
    assert_eq!(
        source.matches("_host_req.event_data = \"\";").count(),
        3,
        "each BasicHTTP request's event data is empty:\n{source}"
    );
    let hosts = [
        "--host-processor",
        "x-sce-host",
        "--host-invoker",
        "x-app-host",
    ];
    let (_, served) = generated(SCRIPTED, &[], &hosts);
    assert!(
        served.contains("return JSON.stringify(_scxml_params(table.unpack(_sce_send_pairs)))")
            && served.contains("_host_req.event_data = _ed;"),
        "a send a host serves is handed the payload it builds:\n{served}"
    );
}

/// The same under `sce-static`: a BasicHTTP send writes each pair as the text
/// its form carries and nothing else, from the one value the machine's fields
/// give it: no JSON object is begun in the scratch's payload, no `_ed` is
/// declared, and a text that does not fit its row refuses the send, never cut.
#[test]
fn a_static_basic_http_send_writes_its_pairs_as_text_only() {
    let doc = std::fs::read_to_string(
        repo_root().join("sce-build/tests/fixtures/static_datamodel/static_send_http.scxml"),
    )
    .expect("the BasicHTTP machine");
    let (_, source) = generated(&doc, &[], &[]);
    assert!(
        !source.contains("const char *_ed") && !source.contains("sce_forge_wire_begin(&sce_wire_"),
        "a BasicHTTP send builds no payload:\n{source}"
    );
    assert!(
        source.contains("if (sce_forge_wire_text(sce_value, SCE_SCRATCH_BUF(pairs[")
            && source.contains(
                "\"<send> <param name='count'> does not fit a request's parameter, so the \
                 message was not sent\""
            ),
        "each pair is written as text, refused whole past its row:\n{source}"
    );
}
