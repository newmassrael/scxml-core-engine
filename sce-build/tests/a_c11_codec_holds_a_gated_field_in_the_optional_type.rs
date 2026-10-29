// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A positional codec generated for C11 holds a field that `sce:present-if` gates in
//! the optional value of its element (docs/adr/0055, step 14), as every other
//! backend does: a number or an enum in the struct `sce/forge/optional.h` declares for
//! it, a byte string or a text in the one over its bytes and their length, and a list,
//! a TLV chain or an embedded codec in a member of the same shape (`present` and
//! `value`) over the entries and their count, or the body.
//!
//! The gate is the value, not the carrier: the decoder fills `present` from the
//! predicate, and the encoder writes the field where `present` says so and refuses a
//! message whose chain-membership predicate contradicts it. Two things a plain member
//! could not hold apart are held here: a value that is present and zero or empty, and a
//! value that is absent.
//!
//! Each case generates a document for C11 and holds the text of what is written; the
//! last compiles a driver against the generated headers (`-std=c11 -Wall -Wextra
//! -Werror`) and runs it, so that the shapes are the ones a program can use.

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

/// Generate the repository's `document` for C11 into `out_dir`.
fn generate_into(document: &str, out_dir: &Path) {
    let out = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("generate")
        .arg(repo_root().join(document))
        .args(["-l", "c"])
        .arg("-o")
        .arg(out_dir)
        .output()
        .expect("invoke sce-codegen");
    assert!(
        out.status.success(),
        "generate {document} -l c:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// What `generate -l c` writes for `document`, every file of it in one text.
fn generated_c(document: &str) -> String {
    let out_dir = tempdir().expect("tempdir");
    generate_into(document, out_dir.path());
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

const BASIC: &str = "tests/forge/resources/codec_present_if_basic.scxml";
const VLE: &str = "tests/forge/resources/codec_present_if_vle.scxml";
const TAIL: &str = "tests/forge/resources/codec_present_if_tail.scxml";
const STRING: &str = "tests/forge/resources/codec_present_if_string.scxml";
const REPEAT_ELEM: &str = "tests/forge/resources/codec_repeat_elem.scxml";
const REPEAT: &str = "tests/forge/resources/codec_repeat_present_if_basic.scxml";
const TLV_CHAIN: &str = "tests/forge/resources/codec_tlv_chain_present_if_basic.scxml";
const EMBED: &str = "tests/forge/resources/codec_zenoh_undecl_subscriber.scxml";

/// A gated number is the optional value of its type: read into `some`, written where
/// it is present through `value`, and a gated variable-width number the same.
#[test]
fn a_gated_number_is_the_optional_struct_of_its_type() {
    let text = generated_c(BASIC);
    holds(
        "codec_present_if_basic",
        &text,
        &[
            "#include \"sce/forge/optional.h\"",
            "sce_optional_uint16_t seq;",
            "out->seq = sce_optional_uint16_some(",
            "out->seq = sce_optional_uint16_none();",
            "if (self->seq.present) {",
        ],
        &["out->seq = 0;", "if ((self->flags & 0x01) != 0) {"],
    );
    let text = generated_c(VLE);
    holds(
        "codec_present_if_vle",
        &text,
        &[
            "out->optional_id = sce_optional_uint64_some(_v);",
            "if (self->optional_id.present) {",
        ],
        &[],
    );
}

/// A gated byte string or text is the optional value over its bytes and their length:
/// the struct of `sce_static_bytes_<N>_t`, whose `data` is not NUL-terminated.
#[test]
fn a_gated_byte_string_or_text_is_the_optional_struct_over_its_bytes() {
    let text = generated_c(TAIL);
    holds(
        "codec_present_if_tail",
        &text,
        &[
            "SCE_OPTIONAL_DECLARE_BYTES(64)",
            "sce_optional_bytes_64_t payload;",
            "memcpy(out->payload.value.data, raw, _n);",
            "out->payload.value.len = _n;",
            "out->payload.present = true;",
            "out->payload = sce_optional_bytes_64_none();",
            "if (self->payload.present) {",
            "self->payload.value.data, self->payload.value.len",
        ],
        &["out->payload_len = 0;", "uint8_t payload[64];"],
    );
    let text = generated_c(STRING);
    holds(
        "codec_present_if_string",
        &text,
        &[
            "sce_optional_bytes_32_t text;",
            "sce_forge_is_valid_utf8(raw, _n)",
            "out->text.value.len = _n;",
            "if (self->text.present) {",
        ],
        &["char text[", "out->text_len = _n;"],
    );
}

/// A gated list, a gated TLV chain and a gated embedded codec are members of the same
/// shape: `present`, and `value` over the entries and their count, or the body.
#[test]
fn a_gated_list_chain_or_embed_is_a_member_with_present_and_value() {
    let text = generated_c(REPEAT);
    holds(
        "codec_repeat_present_if_basic",
        &text,
        &[
            "struct { bool present; struct { codec_repeat_elem_t data[",
            "size_t len; } value; } elems;",
            "out->elems.value.len = _n;",
            "out->elems.present = true;",
            "memset(&out->elems, 0, sizeof out->elems);",
            "if (self->elems.present) {",
            "self->elems.value.len",
        ],
        &["out->elems_len"],
    );
    let text = generated_c(TLV_CHAIN);
    holds(
        "codec_tlv_chain_present_if_basic",
        &text,
        &[
            "} value; }",
            "out->entries.present = true;",
            "if (self->entries.present) {",
        ],
        &["out->entries_len"],
    );
    let text = generated_c(EMBED);
    holds(
        "codec_zenoh_undecl_subscriber",
        &text,
        &[
            "struct { bool present; codec_zenoh_decl_ext_keyexpr_t value; } ext_keyexpr;",
            "out->ext_keyexpr.present = true;",
            "if (self->ext_keyexpr.present) {",
            "&self->ext_keyexpr.value",
        ],
        &[],
    );
}

/// A chain-membership predicate that contradicts a gated field is refused, as every other
/// backend refuses it, with the status the runtime declares for it.
#[test]
fn a_gated_field_a_chain_contradicts_is_refused_on_encode() {
    let text = generated_c("tests/forge/resources/codec_chain_has_marker.scxml");
    holds(
        "codec_chain_has_marker",
        &text,
        &["return SCE_FORGE_CODEC_PRESENT_IF_MISMATCH;"],
        &[],
    );
}

/// The driver: a gated number, byte string and list, each told apart where it is
/// absent, present and zero or empty, and present with a value, through decode and
/// encode, with the carrier flag held as the caller holds it.
const DRIVER: &str = r#"
#include <stdio.h>
#include <string.h>

#include "codec_present_if_basic.h"
#include "codec_present_if_tail.h"
#include "codec_repeat_present_if_basic.h"

static int failures = 0;

#define CHECK(condition, what)                                                  \
    do {                                                                        \
        if (!(condition)) {                                                     \
            fprintf(stderr, "FAIL %s\n", what);                                 \
            ++failures;                                                         \
        }                                                                       \
    } while (0)

int main(void) {
    uint8_t out[64];
    size_t n = 0;

    /* A number. */
    {
        const uint8_t on[] = {0x01, 0x12, 0x34};
        codec_present_if_basic_t v;
        memset(&v, 0, sizeof v);
        sce_forge_cursor_t c = sce_forge_cursor_init(on, sizeof on);
        CHECK(codec_present_if_basic_decode(&c, &v) == SCE_FORGE_CODEC_OK, "number decode");
        CHECK(v.flags == 1 && v.seq.present && v.seq.value == 0x1234, "number present");
        CHECK(codec_present_if_basic_encode_to_buf(&v, out, sizeof out, &n) == SCE_FORGE_CODEC_OK &&
                  n == sizeof on && memcmp(out, on, n) == 0,
              "number round trip");
        /* The carrier flag is a field of its own; the value is written where it is present. */
        v.flags = 0;
        CHECK(codec_present_if_basic_encode_to_buf(&v, out, sizeof out, &n) == SCE_FORGE_CODEC_OK &&
                  n == 3 && out[0] == 0x00 && out[1] == 0x12 && out[2] == 0x34,
              "number written where present, whatever the flag");
    }
    {
        const uint8_t zero[] = {0x01, 0x00, 0x00};
        codec_present_if_basic_t v;
        memset(&v, 0, sizeof v);
        sce_forge_cursor_t c = sce_forge_cursor_init(zero, sizeof zero);
        CHECK(codec_present_if_basic_decode(&c, &v) == SCE_FORGE_CODEC_OK, "zero decode");
        CHECK(v.seq.present && v.seq.value == 0, "a present zero is a value");
    }
    {
        const uint8_t off[] = {0x00};
        codec_present_if_basic_t v;
        memset(&v, 0xAB, sizeof v);
        sce_forge_cursor_t c = sce_forge_cursor_init(off, sizeof off);
        CHECK(codec_present_if_basic_decode(&c, &v) == SCE_FORGE_CODEC_OK, "number absent decode");
        CHECK(!v.seq.present && v.seq.value == 0, "number absent");
        CHECK(codec_present_if_basic_encode_to_buf(&v, out, sizeof out, &n) == SCE_FORGE_CODEC_OK &&
                  n == 1 && out[0] == 0x00,
              "number absent round trip");
    }

    /* A byte string: empty is a value, and absent is not. */
    {
        const uint8_t empty[] = {0x01};
        codec_present_if_tail_t v;
        memset(&v, 0, sizeof v);
        sce_forge_cursor_t c = sce_forge_cursor_init(empty, sizeof empty);
        CHECK(codec_present_if_tail_decode(&c, &v) == SCE_FORGE_CODEC_OK, "empty decode");
        CHECK(v.payload.present && v.payload.value.len == 0, "present and empty");
        CHECK(codec_present_if_tail_encode_to_buf(&v, out, sizeof out, &n) == SCE_FORGE_CODEC_OK &&
                  n == 1 && out[0] == 0x01,
              "empty round trip");
    }
    {
        const uint8_t full[] = {0x01, 0xAA, 0xBB};
        codec_present_if_tail_t v;
        memset(&v, 0, sizeof v);
        sce_forge_cursor_t c = sce_forge_cursor_init(full, sizeof full);
        CHECK(codec_present_if_tail_decode(&c, &v) == SCE_FORGE_CODEC_OK, "bytes decode");
        CHECK(v.payload.present && v.payload.value.len == 2 && v.payload.value.data[0] == 0xAA &&
                  v.payload.value.data[1] == 0xBB,
              "bytes present");
        CHECK(codec_present_if_tail_encode_to_buf(&v, out, sizeof out, &n) == SCE_FORGE_CODEC_OK &&
                  n == sizeof full && memcmp(out, full, n) == 0,
              "bytes round trip");
    }
    {
        const uint8_t off[] = {0x00};
        codec_present_if_tail_t v;
        memset(&v, 0, sizeof v);
        sce_forge_cursor_t c = sce_forge_cursor_init(off, sizeof off);
        CHECK(codec_present_if_tail_decode(&c, &v) == SCE_FORGE_CODEC_OK, "bytes absent decode");
        CHECK(!v.payload.present && v.payload.value.len == 0, "bytes absent");
        CHECK(codec_present_if_tail_encode_to_buf(&v, out, sizeof out, &n) == SCE_FORGE_CODEC_OK &&
                  n == 1 && out[0] == 0x00,
              "bytes absent round trip");
    }

    /* A list: present and empty is a value, and absent is not. */
    {
        const uint8_t empty[] = {0x01, 0x00};
        codec_repeat_present_if_basic_t v;
        memset(&v, 0, sizeof v);
        sce_forge_cursor_t c = sce_forge_cursor_init(empty, sizeof empty);
        CHECK(codec_repeat_present_if_basic_decode(&c, &v) == SCE_FORGE_CODEC_OK, "empty list");
        CHECK(v.elems.present && v.elems.value.len == 0, "a list on the wire and empty");
        CHECK(codec_repeat_present_if_basic_encode_to_buf(&v, out, sizeof out, &n) ==
                      SCE_FORGE_CODEC_OK &&
                  n == sizeof empty && memcmp(out, empty, n) == 0,
              "empty list round trip");
    }
    {
        const uint8_t two[] = {0x01, 0x02, 0xBE, 0xEF, 0xFA, 0xCE};
        codec_repeat_present_if_basic_t v;
        memset(&v, 0, sizeof v);
        sce_forge_cursor_t c = sce_forge_cursor_init(two, sizeof two);
        CHECK(codec_repeat_present_if_basic_decode(&c, &v) == SCE_FORGE_CODEC_OK, "list decode");
        CHECK(v.elems.present && v.elems.value.len == 2 && v.elems.value.data[0].seq == 0xBEEF &&
                  v.elems.value.data[1].seq == 0xFACE,
              "list present");
        CHECK(codec_repeat_present_if_basic_encode_to_buf(&v, out, sizeof out, &n) ==
                      SCE_FORGE_CODEC_OK &&
                  n == sizeof two && memcmp(out, two, n) == 0,
              "list round trip");
    }
    {
        const uint8_t off[] = {0x00};
        codec_repeat_present_if_basic_t v;
        memset(&v, 0, sizeof v);
        sce_forge_cursor_t c = sce_forge_cursor_init(off, sizeof off);
        CHECK(codec_repeat_present_if_basic_decode(&c, &v) == SCE_FORGE_CODEC_OK, "list absent");
        CHECK(!v.elems.present && v.elems.value.len == 0, "a list that was not on the wire");
        CHECK(codec_repeat_present_if_basic_encode_to_buf(&v, out, sizeof out, &n) ==
                      SCE_FORGE_CODEC_OK &&
                  n == 1 && out[0] == 0x00,
              "list absent round trip");
    }

    return failures == 0 ? 0 : 1;
}
"#;

/// The generated headers compile as C11 with the strictest warnings and a program that
/// reads and writes a gated number, byte string and list through `present` and `value`
/// tells absent from present and empty, and round-trips each frame.
#[test]
fn a_gated_field_is_held_apart_where_it_is_absent_or_empty() {
    let dir = tempdir().expect("tempdir");
    for document in [BASIC, TAIL, REPEAT_ELEM, REPEAT] {
        generate_into(document, dir.path());
    }
    let driver = dir.path().join("driver.c");
    std::fs::write(&driver, DRIVER).expect("write the driver");
    let binary = dir.path().join("driver");
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
    let backends_c = repo_root().join("backends/c");
    let compiled = Command::new(&cc)
        .args(["-std=c11", "-Wall", "-Wextra", "-Werror", "-I"])
        .arg(dir.path())
        .arg("-I")
        .arg(backends_c.join("forge-runtime/include"))
        .arg("-I")
        .arg(backends_c.join("runtime/include"))
        .arg(&driver)
        .arg("-o")
        .arg(&binary)
        .output()
        .expect("the C compiler runs");
    assert!(
        compiled.status.success(),
        "the driver compiles against the generated headers:\n{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let run = Command::new(&binary).output().expect("the driver runs");
    assert!(
        run.status.success(),
        "the driver's checks hold:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
}
