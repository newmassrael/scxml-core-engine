// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A C11 `sce-static` machine reads the payload of an event whose schema holds
//! a composition, at any depth (docs/adr/0086, section 6), and sends a
//! composed value by its name (section 5.2), as the Rust, Kotlin, Go, Python
//! and C++ machines do, with no allocator and no closure: its payload struct
//! holds each field in a type named by its shape and declared once under a
//! guard of that name (section 7.2), the lift reads each field where the
//! machine holds the payload, by one function of the machine per type over
//! the runtime's bounded readers (section 4), and names the path of a refused
//! place in a buffer of the longest path the type has, the inject seam writes
//! what a host hands it part by part, each part held to its type as it is
//! written (sections 3.1 and 5.4), and the statechart's reads, copies and
//! sends lower to the machine's own types.
//!
//! # What is held
//!
//! * The payload struct, the record a list of it holds, and the list of each
//!   shape, each in the type named by its shape, the record declared once.
//! * The lift: a function per type, the bound and the path of each, the
//!   defaults of the list the second revision added, the reason that names the
//!   path at its longest, and the payload filled where the machine holds it.
//! * The inject seam: each pair named and written part by part, its keys
//!   ascending, a list past its room refused.
//! * The reads: `<foreach>` over a list the payload holds and over a list an
//!   enclosing loop's item holds, each the copy the loop begins with, and an
//!   element of a list of lists and of a list of optional values read where an
//!   index outside the list reads as empty and as absent.
//! * A list taken whole into a variable, by assignment where the two are one
//!   type and by the constructor of the variable's type where its bound is
//!   larger.
//! * A `<send>` that names each composed field's place, each written where
//!   its pair stands, and one that gives a field of the first revision no
//!   value, refused.
//! * A text a composed payload holds, read as a value, refused by name on
//!   C11, which holds it in its buffer; and a payload that holds a union,
//!   refused by name on C11, which holds no union yet (docs/adr/0092, section 7).

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::{compile_scxml_lang_typed, find_template_dir_for};
use tempfile::tempdir;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

fn fixtures() -> PathBuf {
    repo_root().join("sce-build/tests/fixtures/static_datamodel")
}

/// The documents a reader of `batch.sent` imports, copied beside it.
const SIBLINGS: [&str; 4] = [
    "enum_signal_level.scxml",
    "event_batch_item.scxml",
    "event_batch.scxml",
    "event_counted.scxml",
];

/// A reader of `batch.sent` that measures a list at each depth, walks the
/// payload's `items` with an index and each item's `tags`, walks the payload's
/// list of lists and each of its lists, and reads an element that may be
/// absent.
const READER: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="batch_reader">
  <sce:import kind="event-schema" src="event_batch.scxml" as="Batch"/>
  <sce:import kind="event-schema" src="event_batch_item.scxml" as="Item"/>
  <sce:import kind="enum" src="enum_signal_level.scxml" as="SignalLevel"/>
  <datamodel>
    <data id="seen" sce:type="uint32" expr="0" sce:direction="out"/>
    <data id="cell" sce:type="uint8" expr="0" sce:direction="out"/>
    <data id="first_maybe" sce:type="optional&lt;uint16&gt;" expr="null"/>
  </datamodel>
  <state id="idle">
    <transition event="batch.sent" target="idle">
      <assign location="seen"
              expr="len(_event.data.items) + len(_event.data.grid) + len(_event.data.levels)"/>
      <foreach array="_event.data.items" item="row" index="i">
        <assign location="seen" expr="seen + len(row.tags) + len(_event.data.items[i].tags)"/>
        <foreach array="row.tags" item="word">
          <assign location="seen" expr="seen + 1"/>
        </foreach>
      </foreach>
      <foreach array="_event.data.grid" item="cells">
        <foreach array="cells" item="n">
          <assign location="cell" expr="n"/>
        </foreach>
      </foreach>
      <if cond="len(_event.data.grid) &gt; 1 &amp;&amp; len(_event.data.grid[1]) &gt; 0">
        <assign location="cell" expr="_event.data.grid[1][0]"/>
      </if>
      <if cond="len(_event.data.maybe) &gt; 0">
        <assign location="first_maybe" expr="_event.data.maybe[0]"/>
      </if>
    </transition>
  </state>
</scxml>
"#;

/// The C11 machine of `document`, written as `name` beside the batch schemas,
/// or why it is not generated: every file it is generated as, the header and
/// the source, one after the other.
fn c11_generated(name: &str, document: &str) -> Result<String, String> {
    let dir = tempdir().expect("tempdir");
    for sibling in SIBLINGS {
        std::fs::copy(fixtures().join(sibling), dir.path().join(sibling)).expect("copy a sibling");
    }
    let path = dir.path().join(name);
    std::fs::write(&path, document).expect("write the document");
    let language = sce_build::generator::Language::C11;
    let output = compile_scxml_lang_typed(
        path.to_str().expect("a utf-8 path"),
        &find_template_dir_for(language),
        language,
    )
    .map_err(|e| format!("{e:?}"))?;
    let names: Vec<String> = output.files.iter().map(|(name, _)| name.clone()).collect();
    assert!(
        names.iter().any(|name| name.ends_with("_sm.h")),
        "the machine's header is generated among {names:?}"
    );
    Ok(output
        .files
        .into_iter()
        .map(|(_, code)| code)
        .collect::<Vec<_>>()
        .join("\n"))
}

/// The C11 machine of `document`, written as `name` beside the batch schemas.
fn c11_machine(name: &str, document: &str) -> String {
    c11_generated(name, document).unwrap_or_else(|e| panic!("C11 lowers {name}: {e}"))
}

/// The C11 machine of [`READER`].
fn reader_machine() -> String {
    c11_machine("batch_reader.scxml", READER)
}

/// The C11 machine of the fixture `name`.
fn fixture_machine(name: &str) -> String {
    let text = std::fs::read_to_string(fixtures().join(name)).expect("fixture");
    c11_machine(name, &text)
}

/// `code` holds each of `lines`.
fn holds(code: &str, lines: &[&str]) {
    for line in lines {
        assert!(code.contains(line), "expected `{line}` in:\n{code}");
    }
}

/// Pinned by `C11Composed::record_type`, `C11Composed::list_type`,
/// `C11Composed::optional_type`, `C11Composed::text_type`,
/// `C11Composed::bytes_type` and `C11Composed::record_def`
/// (`sce-build/src/forge/composed_lowering/c11.rs`), by `c11_list_type` and
/// `c11_list_def` (`sce-build/src/forge/static_lowering.rs`), by
/// `c11_composed_payload` (`sce-build/src/forge/generator.rs`), and by the
/// declaration of each record a payload names (`declare_composed_payload`).
#[test]
fn the_payload_is_held_in_types_named_by_their_shape_each_declared_once() {
    let code = reader_machine();
    holds(
        &code,
        &[
            "typedef struct batch_reader_batch_sent_payload {\n    \
             batch_reader_record_item_list_3_t items;\n    sce_static_list_uint8_2_list_2_t \
             grid;\n    enum_signal_level_enum_list_2_t levels;\n    sce_optional_uint16_list_2_t \
             maybe;\n    sce_optional_record_sce_static_list_uint8_2_t spare;\n    \
             sce_static_string_4_t note;\n    sce_static_list_uint8_2_t late;\n} \
             batch_reader_batch_sent_payload_t;",
            "typedef struct {\n    uint32_t id;\n    int64_t amount;\n    sce_static_string_8_t \
             label;\n    enum_signal_level_enum_t level;\n    sce_optional_int64_t due;\n    \
             sce_static_bytes_4_t blob;\n    sce_static_string_8_list_4_t tags;\n} \
             batch_reader_record_item_t;",
            "#ifndef BATCH_READER_RECORD_ITEM_LIST_3\n",
            "typedef struct {\n    size_t len;\n    batch_reader_record_item_t data[3];\n} \
             batch_reader_record_item_list_3_t;",
            "typedef struct {\n    size_t len;\n    sce_static_list_uint8_2_t data[2];\n} \
             sce_static_list_uint8_2_list_2_t;",
            "SCE_OPTIONAL_DECLARE_RECORD(record_sce_static_list_uint8_2, sce_static_list_uint8_2_t)",
        ],
    );
    assert_eq!(
        code.matches("} batch_reader_record_item_t;").count(),
        1,
        "a record is declared once:\n{code}"
    );
}

/// Pinned by `C11Composed::read_list`, `C11Composed::read_record`,
/// `C11Composed::read_text`, `C11Composed::read_optional` and
/// `C11Composed::read_enum` (`sce-build/src/forge/composed_lowering/c11.rs`),
/// by the lift `c11_composed_payload` writes and `c11_path_len`, by the row
/// `lift_place_refused` (`sce-build/src/forge/c11_reasons.rs`), and by
/// `composed_defaults_wire` (`sce-build/src/forge/schema_revision.rs`): each
/// field read by the function of its type into the payload the machine holds,
/// a text held to its bound at the top level too, a list the second revision
/// added read as empty, the path of a refused place named whole.
#[test]
fn the_lift_reads_each_field_by_the_function_of_its_type() {
    let code = reader_machine();
    holds(
        &code,
        &[
            r#"const char *_refusal = sce_payload_decode_with_defaults(_data, "{\"late\":[]}", &_fields);"#,
            // `items/2/tags/3` is the longest path: 14 bytes and the NUL.
            "    char sce_path[15u];\n    const size_t sce_path_cap = sizeof(sce_path);\n",
            "        void *const sce_into = &out->items;\n",
            "_refusal = sce_payload_field(&_fields, \"items\", &sce_at);",
            "_refusal = sce_composed_read_batch_reader_record_item_list_3(sce_at, sce_into, \
             sce_path, sce_path_cap);",
            "snprintf(_message, _message_cap, \"`batch.sent` payload: '%.14s' %s\", sce_path, \
             _refusal);",
            "sce_payload_list_open_within(sce_value, 0u, 3u, &sce_cursor)",
            "return sce_payload_value_text(sce_value, sce_out->data, 8u);",
            "return sce_payload_value_text(sce_value, sce_out->data, 4u);",
            "return sce_payload_value_bytes(sce_value, sce_out->data, 4u, &sce_out->len);",
            "static const char *const sce_names[] = { \"low\", \"high\", \"critical\", };",
            "sce_refusal = sce_payload_field(&sce_fields, \"amount\", &sce_at);",
            "sce_refusal = sce_payload_value_i64(sce_at, sce_into);",
            "if (sce_payload_is_null(sce_value)) {",
            "memset(&sm->pending_payload.as.batch_sent, 0, sizeof(sm->pending_payload.as.batch_sent));",
            "if (!batch_reader_lift_batch_sent(sm, evt->data, &sm->pending_payload.as.batch_sent, \
             _message,",
        ],
    );
    // A function per type, defined once however many places read it.
    assert_eq!(
        code.matches("static const char *sce_composed_read_sce_static_string_8(")
            .count(),
        1,
        "{code}"
    );
}

/// Pinned by `C11Composed::write_list`, `C11Composed::write_record`,
/// `C11Composed::write_optional` and `c11_pairs`
/// (`sce-build/src/forge/composed_lowering/c11.rs`), `sce_forge_wire_pair_key`
/// and `sce_forge_wire_refuse` (`sce/forge/wire.h`), and the seam
/// `c11_composed_payload` writes (`sce-build/src/forge/generator.rs`): the seam
/// writes what a host hands it as its JSON part by part, its keys ascending,
/// each part held to its type as it is written (docs/adr/0086, section 5.4).
#[test]
fn the_inject_seam_writes_each_part_held_to_its_type() {
    let code = reader_machine();
    holds(
        &code,
        &[
            "bool batch_reader_raise_batch_sent_typed(batch_reader_t *sm, const \
             batch_reader_batch_sent_payload_t *payload);",
            "    sce_forge_wire_t *const sce_wire = &sce_wire_;\n    \
             sce_forge_wire_begin(sce_wire, out, cap);\n",
            "if (((*payload).late).len > sizeof(((*payload).late).data) / \
             sizeof(((*payload).late).data[0])) {\n    sce_forge_wire_refuse(sce_wire);\n}",
            "if (memchr(((*sce_item).label).data, '\\0', sizeof(((*sce_item).label).data)) == NULL)",
            "const char *const sce_name = enum_signal_level_declared_name((*sce_item));",
            "    if (!sce_forge_wire_end(sce_wire)) {\n        return false;\n    }\n",
        ],
    );
    let at = |key: &str| {
        code.find(&format!(
            "sce_forge_wire_pair_key(sce_wire, \"\\\"{key}\\\"\");"
        ))
        .unwrap_or_else(|| panic!("the pair `{key}` in:\n{code}"))
    };
    let keys = ["grid", "items", "late", "levels", "maybe", "note", "spare"];
    for pair in keys.windows(2) {
        assert!(
            at(pair[0]) < at(pair[1]),
            "`{}` before `{}`:\n{code}",
            pair[0],
            pair[1]
        );
    }
}

/// Pinned by the loop over a list a place holds (`walked_place`,
/// `sce-build/src/forge/static_lowering.rs`) and `CTarget::foreach_loop_typed`,
/// and by the failed read of an index (`c_failed_part`,
/// `sce-build/src/forge/expr.rs`): a list the payload holds and a list an
/// enclosing loop's item holds are each walked as the copy the loop begins
/// with, in the machine's own storage, and an element read through an index
/// outside its list reads as an empty list or an absent value.
#[test]
fn the_reads_lower_at_any_depth() {
    let code = reader_machine();
    holds(
        &code,
        &[
            "&sm->scratch.loop_0.of_batch_reader_record_item_list_3,",
            "(sm->pending_payload.as.batch_sent.items).len",
            "&sm->scratch.loop_1.of_sce_static_string_8_list_4,",
            "&sm->scratch.loop_0.of_sce_static_list_uint8_2_list_2,",
            "&sm->scratch.loop_1.of_sce_static_list_uint8_2,",
            ".tags : (__typeof__(",
            "){ .len = 0 }",
            "){ .present = false }",
        ],
    );
}

/// Pinned by `whole_list_copy` (`sce-build/src/forge/static_lowering.rs`),
/// `copier` and `C11Composed::copy_list_into` (`composed_lowering.rs`,
/// `composed_lowering/c11.rs`), and `c11_list_def`: a list a payload holds is
/// taken whole into a list variable of a larger bound by the constructor of the
/// variable's type, and into one of the same bound, one type, by assignment.
#[test]
fn a_list_is_taken_whole_into_a_variable_of_its_own_type() {
    let wider = fixture_machine("static_composed_payload.scxml");
    holds(
        &wider,
        &[
            "sm->policy.v_late = sce_static_list_uint8_4_of((sm->pending_payload.as.batch_sent.late).data, \
             (sm->pending_payload.as.batch_sent.late).len);",
            "static inline sce_static_list_uint8_4_t sce_static_list_uint8_4_of(const uint8_t *data, \
             size_t len) {",
        ],
    );
    let same = fixture_machine("static_composed_send.scxml");
    holds(
        &same,
        &["sm->policy.v_late = (sm->pending_payload.as.batch_sent.late);"],
    );
}

/// Pinned by `composed_wire_value` and `Param::native_writes`
/// (`sce-build/src/forge/static_lowering.rs`, `sce-build/src/model.rs`), the
/// `native_writes` arm of `send_payload_json`
/// (`tools/codegen/templates/c/state_machine.c.jinja2`) and `C11Composed`'s
/// writers: each composed field of the forwarded delivery is written where its
/// pair stands, by walking its type, beside a literal param written as a pair.
#[test]
fn a_send_writes_each_composed_field_where_its_pair_stands() {
    let code = fixture_machine("static_composed_send.scxml");
    holds(
        &code,
        &[
            "sce_forge_wire_t *const sce_wire = &sce_wire_;\n                        \
             sce_forge_wire_pair_key(sce_wire, \"\\\"items\\\"\");\n",
            "if ((sm->pending_payload.as.batch_sent.items).len > \
             sizeof((sm->pending_payload.as.batch_sent.items).data) / \
             sizeof((sm->pending_payload.as.batch_sent.items).data[0])) {",
            "sce_forge_wire_pair_key(sce_wire, \"\\\"late\\\"\");\nif ((sm->policy.v_late).len > ",
            "if ((sm->pending_payload.as.batch_sent.spare).present) {",
            "const char *const sce_name = enum_signal_level_declared_name((*sce_item));",
        ],
    );
    // A send to the machine itself hands no host its pairs.
    assert!(!code.contains("sce_forge_wire_copy_since("), "{code}");
}

/// Pinned by the `native_writes` arm of `send_payload_json` under `host_texts`
/// (`tools/codegen/templates/c/state_machine.c.jinja2`), `sce_forge_wire_mark`
/// and `sce_forge_wire_copy_since` (`sce/forge/wire.h`), and the lowering of a
/// `<send>`'s params (`sce-build/src/forge/static_lowering.rs`), which refuses
/// no composed param of a send a host serves: a `<send>` to a Mesh peer, which
/// a C11 machine hands its host, writes each composed value into the payload
/// where its pair stands, and hands the host the value's JSON as the pair's
/// text, read back from that payload into the row of the pair.
#[test]
fn a_send_a_host_serves_hands_it_each_composed_value_as_its_json() {
    let text =
        std::fs::read_to_string(fixtures().join("static_composed_send.scxml")).expect("fixture");
    let to_a_peer = text.replace(
        "<send event=\"batch.sent\" target=\"#_internal\">",
        "<send event=\"batch.sent\" target=\"#peers\">",
    );
    assert_ne!(to_a_peer, text, "the fixture sends to the machine itself");
    let code = c11_machine("static_composed_send.scxml", &to_a_peer);
    // The mark is taken after the pair's key and before its value is written,
    // so the text read back is the value alone.
    let key = code
        .find("sce_forge_wire_pair_key(sce_wire, \"\\\"items\\\"\");\n")
        .unwrap_or_else(|| panic!("the pair `items` in:\n{code}"));
    let after_key = &code[key..];
    let mark = after_key
        .find("const size_t sce_value_at = sce_forge_wire_mark(sce_wire);\n")
        .unwrap_or_else(|| panic!("a mark after the pair `items` in:\n{code}"));
    let value = after_key
        .find("if ((sm->pending_payload.as.batch_sent.items).len > ")
        .unwrap_or_else(|| panic!("the value of `items` in:\n{code}"));
    assert!(
        mark < value,
        "the mark comes before the value:\n{after_key}"
    );
    holds(
        &code,
        &[
            "if (sce_forge_wire_copy_since(sce_wire, sce_value_at,\n                                \
             SCE_SCRATCH_BUF(pairs[1]), SCE_SCRATCH_CAP(pairs[1]))) {",
            "_host_params[_host_param_count].name = \"items\";",
            "_host_params[_host_param_count].name = \"spare\";",
            "_host_params[_host_param_count].value = SCE_SCRATCH_BUF(pairs[1]);",
        ],
    );
}

/// `sce-codegen <args> <file>` over `document` written as `name` in a
/// directory that holds `siblings` of `from`, and whether it succeeded, with
/// what it said.
fn run(
    args: &[&str],
    from: &Path,
    siblings: &[&str],
    name: &str,
    document: &str,
) -> (bool, String) {
    let dir = tempdir().expect("tempdir");
    for sibling in siblings {
        std::fs::copy(from.join(sibling), dir.path().join(sibling)).expect("copy a sibling");
    }
    std::fs::write(dir.path().join(name), document).expect("write the document");
    let out = Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(args)
        .arg(dir.path().join(name))
        .output()
        .expect("invoke sce-codegen");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout),
    )
}

/// Pinned by `ComposedLowering::of` (`sce-build/src/forge/event_schema_check.rs`):
/// C11 takes the read, the whole copy and the send of both fixtures.
#[test]
fn c11_takes_the_read_the_copy_and_the_send() {
    for (name, document) in [
        ("batch_reader.scxml", READER.to_string()),
        (
            "static_composed_payload.scxml",
            std::fs::read_to_string(fixtures().join("static_composed_payload.scxml"))
                .expect("fixture"),
        ),
        (
            "static_composed_send.scxml",
            std::fs::read_to_string(fixtures().join("static_composed_send.scxml"))
                .expect("fixture"),
        ),
    ] {
        let (ok, out) = run(
            &["check", "-l", "c11"],
            &fixtures(),
            &SIBLINGS,
            name,
            &document,
        );
        assert!(ok, "C11 lowers {name}:\n{out}");
    }
}

/// Pinned by the gate's `<send>` arm (`first_unlowered_composed_use`,
/// `sce-build/src/forge/event_schema_check.rs`): a composed field of the
/// schema's first revision is carried by every delivery (docs/adr/0086,
/// section 10.1), so a C11 send that gives one no value is refused by name,
/// where a list a later revision adds may be left out (section 10.2).
#[test]
fn a_send_that_gives_a_first_revision_field_no_value_is_refused_on_c11() {
    let text =
        std::fs::read_to_string(fixtures().join("static_composed_send.scxml")).expect("fixture");
    let without_items = text.replace(
        "        <param name=\"items\" expr=\"_event.data.items\"/>\n",
        "",
    );
    assert_ne!(without_items, text, "the fixture gives `items` by its name");
    let (ok, out) = run(
        &["check", "-l", "c11"],
        &fixtures(),
        &SIBLINGS,
        "static_composed_send.scxml",
        &without_items,
    );
    assert!(!ok, "a send without `items` lacks its value:\n{out}");
    assert!(out.contains("generate/unsupported-feature"), "{out}");
    assert!(out.contains("the field `items`"), "{out}");
    let without_late = text.replace("        <param name=\"late\" expr=\"late\"/>\n", "");
    assert_ne!(without_late, text, "the fixture gives `late` by its name");
    let (ok, out) = run(
        &["check", "-l", "c11"],
        &fixtures(),
        &SIBLINGS,
        "static_composed_send.scxml",
        &without_late,
    );
    assert!(ok, "`late` is a list the second revision added:\n{out}");
}

/// Pinned by `Rewrites::composed_text_refused` and `CTarget::reads_composed_text`
/// (`sce-build/src/forge/static_lowering.rs`) and `composed_text_read`
/// (`sce-build/src/forge/expr.rs`): a text a composed payload holds is a buffer
/// of its bound on C11, which no C11 expression reads as a text, so a read of
/// one as a value is refused by name where the machine is generated, rather
/// than written as C that reads a buffer as a pointer; at the top level, at
/// depth 3, and as the item of a loop over a list of texts.
#[test]
fn a_text_a_composed_payload_holds_read_as_a_value_is_refused_by_name_on_c11() {
    for read in [
        r#"<if cond="_event.data.note === 'ab'"><assign location="seen" expr="1"/></if>"#,
        r#"<if cond="len(_event.data.items) &gt; 0 &amp;&amp; _event.data.items[0].label === 'ab'"><assign location="seen" expr="1"/></if>"#,
        r#"<foreach array="_event.data.items" item="row"><foreach array="row.tags" item="word"><if cond="word === 'ab'"><assign location="seen" expr="1"/></if></foreach></foreach>"#,
    ] {
        let document = READER.replace(
            "<transition event=\"batch.sent\" target=\"idle\">",
            &format!("<transition event=\"batch.sent\" target=\"idle\">\n      {read}"),
        );
        let Err(refusal) = c11_generated("batch_reader.scxml", &document) else {
            panic!("C11 reads no text a composed payload holds as a value: {read}");
        };
        assert!(
            refusal.contains("a text or a byte string a composed value holds, read as a value")
                && refusal.contains("has no C11 lowering yet"),
            "{read}: {refusal}"
        );
    }
    // The same document reads its numbers and lengths, which C11 lowers.
    reader_machine();
}

/// A reader of `shapes.sent` that tests the tag of the payload's `one`,
/// importing the tag's enum, the event and the records its cases hold.
const SHAPES_READER: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="shapes_reader">
  <sce:import kind="enum" src="enum_shape_kind.scxml" as="Kind"/>
  <sce:import kind="event-schema" src="event_shapes.scxml" as="Shapes"/>
  <sce:import kind="event-schema" src="shape_radius.scxml" as="Radius"/>
  <sce:import kind="event-schema" src="shape_rect.scxml" as="Rect"/>
  <sce:import kind="event-schema" src="shape_label.scxml" as="Label"/>
  <sce:import kind="event-schema" src="shape_path.scxml" as="Path"/>
  <datamodel>
    <data id="seen" sce:type="uint32" expr="0"/>
  </datamodel>
  <state id="idle">
    <transition event="shapes.sent" target="idle">
      <if cond="_event.data.one.kind === Kind.circle">
        <assign location="seen" expr="seen + 1"/>
      </if>
    </transition>
  </state>
</scxml>
"#;

/// The union fixtures a reader of `shapes.sent` imports, beside the fixtures
/// of the static data model (`static_union.scxml` reads them there).
const SHAPES: [&str; 7] = [
    "enum_shape_kind.scxml",
    "shape_radius.scxml",
    "shape_rect.scxml",
    "shape_label.scxml",
    "shape_path.scxml",
    "union_shape.scxml",
    "event_shapes.scxml",
];

/// Pinned by the union arm of the gate's read (`first_unlowered_composed_use`,
/// `sce-build/src/forge/event_schema_check.rs`) and by `union_reached`
/// (`sce-build/src/forge/composed_lowering.rs`): C11 reads a composed payload,
/// holds no union, and so refuses a read of a payload that holds one by its
/// own name, naming the field and the union.
#[test]
fn a_payload_that_holds_a_union_stays_refused_by_name_on_c11() {
    let (ok, out) = run(
        &["check", "-l", "c11"],
        &fixtures(),
        &SHAPES,
        "shapes_reader.scxml",
        SHAPES_READER,
    );
    assert!(!ok, "C11 holds no union yet:\n{out}");
    for part in [
        "generate/unsupported-feature",
        "the field `one` holds `union:Shape`",
        "a statechart's read of `_event.data` has no c11 lowering of its union",
    ] {
        assert!(out.contains(part), "expected `{part}` in:\n{out}");
    }
}
