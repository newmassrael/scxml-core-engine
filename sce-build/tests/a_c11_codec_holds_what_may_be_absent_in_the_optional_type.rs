// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A CBOR or content-line codec generated for C11 holds an entry or a parameter that
//! may be absent in the struct `sce/forge/optional.h` declares for its element
//! (docs/adr/0055, section 8), the one a statechart's variable of the same
//! type is held in, and no `<name>_present` member stands beside a value.
//!
//! What is held here is the generated text: the member each entry is declared as, the
//! declaration each element's struct needs first, and the statements that read and
//! write an entry through `present` and `value`. That the generated header compiles and
//! its bytes are the wire's is held by the C11 conformance lane, which builds each
//! codec fixture and replays its vectors (`backends/c/forge-runtime/tests/conformance`).

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

/// What `generate -l c` writes for the repository's `document`, every file of it in
/// one text.
fn generated_c(document: &str) -> String {
    let out_dir = tempdir().expect("tempdir");
    let out = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("generate")
        .arg(repo_root().join(document))
        .args(["-l", "c"])
        .arg("-o")
        .arg(out_dir.path())
        .output()
        .expect("invoke sce-codegen");
    assert!(
        out.status.success(),
        "generate {document} -l c:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let mut text = String::new();
    let mut pending = vec![out_dir.path().to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).expect("the output directory") {
            let path = entry.expect("an entry").path();
            if path.is_dir() {
                pending.push(path);
            } else {
                text.push_str(&std::fs::read_to_string(&path).unwrap_or_default());
            }
        }
    }
    text
}

/// Every fragment of `wanted` is in `text`, and none of `unwanted` is.
fn holds(what: &str, text: &str, wanted: &[&str], unwanted: &[&str]) {
    for fragment in wanted {
        assert!(
            text.contains(fragment),
            "{what}: the generated codec holds `{fragment}`:\n{text}"
        );
    }
    for fragment in unwanted {
        assert!(
            !text.contains(fragment),
            "{what}: the generated codec holds no `{fragment}`:\n{text}"
        );
    }
}

const CBOR_MAP: &str = "tests/forge/resources/codec_cbor_map.scxml";
const CONTENT_LINE_EVENT: &str = "tests/forge/resources/codec_content_line_event.scxml";
const CONTENT_LINE_ENUMS: &str = "tests/forge/resources/codec_content_line_enums.scxml";
const ENVELOPE: &str = "stdlib/mesh/envelope.scxml";

/// `deadline` is an `optional<uint64>` and `urgent` an `optional<bool>`: each is the
/// runtime's struct of its element, present when `present` says so, read into and
/// written from `value`. The entries of a bare type keep the fixed array and the length
/// beside it, and no entry has a `_present` member.
#[test]
fn a_cbor_entry_that_may_be_absent_is_the_optional_struct_of_its_element() {
    let text = generated_c(CBOR_MAP);
    holds(
        "codec_cbor_map",
        &text,
        &[
            "#include \"sce/forge/optional.h\"",
            "sce_optional_uint64_t deadline;",
            "sce_optional_bool_t urgent;",
            "s = sce_forge_cbor_read_uint(&c, &v->deadline.value);",
            "s = sce_forge_cbor_read_bool(&c, &v->urgent.value);",
            "v->deadline.present = true;",
            "v->urgent.present = true;",
            "if (self->deadline.present) {",
            "if (self->urgent.present) {",
            "s = sce_forge_cbor_write_uint(w, (uint64_t)self->deadline.value);",
            "s = sce_forge_cbor_write_bool(w, self->urgent.value);",
            // A bare type: the array and its length, as it was.
            "char source[64];",
            "size_t source_len;",
            "uint8_t id[16];",
            "s = sce_forge_cbor_read_text(&c, 64, v->source, &v->source_len);",
        ],
        &["deadline_present", "urgent_present"],
    );
}

/// An entry of the envelope that may be absent is the struct of its element: a text the
/// buffer of its bound, an exact-length byte string that buffer and its length, an enum
/// the machine's name for the document's type, a number its own. A text that carries the
/// byte a C string ends at does not fit it, which decode refuses by name rather than cut
/// the text short.
#[test]
fn an_optional_text_bytes_and_enum_entry_hold_what_a_statechart_variable_does() {
    let text = generated_c(ENVELOPE);
    holds(
        "envelope",
        &text,
        &[
            "sce_optional_string_262144_t subject;",
            "sce_optional_bytes_16_t correlation_id;",
            "sce_optional_uint64_t deadline_unix_ms;",
            "sce_optional_enum_rpc_status_t rpc_status;",
            "SCE_OPTIONAL_DECLARE_STRING(262144)",
            "SCE_OPTIONAL_DECLARE_BYTES(16)",
            "typedef RpcStatus_t rpc_status_enum_t;",
            "SCE_OPTIONAL_DECLARE(enum_rpc_status, rpc_status_enum_t, SCE_OPTIONAL_SAME_VALUE)",
            // The text is the buffer of its bound, with no length beside it.
            "s = sce_forge_cbor_read_text(&c, 262144, v->subject.value.data, &text_len);",
            "if (strlen(v->subject.value.data) != text_len) {",
            "s = sce_forge_cbor_write_text(w, self->subject.value.data, strlen(self->subject.value.data));",
            // The byte string is that buffer and its length.
            "s = sce_forge_cbor_read_bytes_exact(&c, 16, v->correlation_id.value.data);",
            "v->correlation_id.value.len = 16;",
            "if (self->correlation_id.value.len != 16U) {",
            "s = sce_forge_cbor_write_bytes(w, self->correlation_id.value.data, 16);",
            "v->rpc_status.present = true;",
            // A bare type keeps its array and length.
            "char source[262144];",
        ],
        &[
            "subject_present",
            "correlation_id_present",
            "rpc_status_present",
            "subject_len",
            "correlation_id_len",
        ],
    );
    // Declared once however many entries hold it: six texts of one bound, three byte
    // strings of one.
    assert_eq!(
        text.matches("#define SCE_OPTIONAL_STRING_262144").count(),
        1,
        "{text}"
    );
    assert_eq!(
        text.matches("#define SCE_OPTIONAL_BYTES_16").count(),
        1,
        "{text}"
    );
}

/// A content-line entry and a parameter that may be absent are the struct of their
/// element, read and written through `present` and `value`; a text is the buffer of its
/// bound, read into it with the length kept in a local and written with the length it
/// ends at. A list, which has its count, and a required entry are as they were.
#[test]
fn a_content_line_entry_and_parameter_that_may_be_absent_are_the_optional_struct() {
    let text = generated_c(CONTENT_LINE_EVENT);
    holds(
        "codec_content_line_event",
        &text,
        &[
            "#include \"sce/forge/optional.h\"",
            "sce_optional_string_24_t summary;",
            "sce_optional_string_160_t description;",
            "sce_optional_string_40_t organizer;",
            "sce_optional_string_24_t organizer_cn;",
            "sce_optional_uint16_t sequence;",
            "sce_optional_int32_t priority;",
            "sce_optional_bool_t all_day;",
            "SCE_OPTIONAL_DECLARE_STRING(24)",
            "SCE_OPTIONAL_DECLARE_STRING(160)",
            // The property's value, then its parameter.
            "sce_forge_cl_property_read_string(&line, 24U, true, v->summary.value.data, &text_len)",
            "sce_forge_cl_property_read_param_string(&line, 24U, v->organizer_cn.value.data, &text_len)",
            "v->summary.present = true;",
            "v->organizer_cn.present = true;",
            "v->sequence.value = (uint16_t)raw_u;",
            "v->priority.value = (int32_t)raw_i;",
            "if (self->summary.present) {",
            "if (self->organizer.present) {",
            "if (self->organizer_cn.present) {",
            "} else if (self->organizer_cn.present) {",
            "sce_forge_cl_writer_param(&out, \"CN\", self->organizer_cn.value.data, strlen(self->organizer_cn.value.data), 24U)",
            "sce_forge_cl_writer_uint(&out, (uint64_t)self->sequence.value)",
            "sce_forge_cl_writer_int(&out, (int64_t)self->priority.value)",
            "sce_forge_cl_writer_bool(&out, self->all_day.value)",
            // A required text and a required parameter keep their arrays and their
            // lengths.
            "sce_forge_cl_property_read_string(&line, 24U, false, v->uid, &v->uid_len)",
            "size_t dtstart_tzid_len;",
            // A list is its rows and its count, never the struct of one value.
            "size_t exdate_count;",
        ],
        &[
            "summary_present",
            "organizer_cn_present",
            "sequence_present",
            "summary_len",
            "organizer_cn_len",
        ],
    );
}

/// An optional enum — a property's value, and a parameter of the lines of a record — is
/// held in the one struct over the machine's name for the document's type, declared once
/// with the alias of that type the machine would declare.
#[test]
fn an_optional_enum_entry_and_parameter_are_declared_once_over_the_machines_type() {
    let text = generated_c(CONTENT_LINE_ENUMS);
    holds(
        "codec_content_line_enums",
        &text,
        &[
            "typedef EnumContentLineStatus_t enum_content_line_status_enum_t;",
            "SCE_OPTIONAL_DECLARE(enum_enum_content_line_status, enum_content_line_status_enum_t, SCE_OPTIONAL_SAME_VALUE)",
            "sce_optional_enum_enum_content_line_status_t status;",
            // The parameter of a line record is a member of the record.
            "sce_optional_enum_enum_content_line_status_t attendee_status;",
            "sce_optional_string_16_t step_note;",
        ],
        &["status_present", "attendee_status_present", "step_note_present"],
    );
    assert_eq!(
        text.matches("#define SCE_OPTIONAL_ENUM_ENUM_CONTENT_LINE_STATUS")
            .count(),
        1,
        "{text}"
    );
}
