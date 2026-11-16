// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Which codecs the C11 backend prints from their encode plan (docs/adr/0081
//! step 4, §6), and what a printed one is.
//!
//! The C11 printer lowers the plan's forms one landing at a time. A codec whose
//! every form it lowers is printed: its header names the table of a source
//! (`{codec}_source_t`, a function for each element the value presents, each
//! handed a context pointer), the value's functions and the table that presents
//! a value as a source (`{codec}_value_source`), and an encode over any source
//! (`{codec}_encode_source`) that derives every other element of the frame
//! (docs/adr/0058 §1); the value's `encode` is the encode over its own table.
//! Every other codec keeps the template's encode whole, and the printer names
//! the first form it does not lower.

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::generator::Language;
use sce_build::{
    c11_codec_printed_file, c11_codec_unlowered_file, compile_forge_file, ForgeCompileOptions,
};
use tempfile::tempdir;

/// The directory of the resources.
fn resources() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/resources")
}

/// Every file the document `path` generates on C11, by name.
fn generated(path: &Path) -> Result<Vec<(String, String)>, String> {
    compile_forge_file(path, Language::C11, &[], &ForgeCompileOptions::default())
        .map(|output| output.files)
        .map_err(|e| format!("{:?}", e.error))
}

/// The C11 header the document `path` generates.
fn header_of(path: &Path) -> Result<String, String> {
    let stem = path
        .file_stem()
        .expect("a document has a stem")
        .to_string_lossy()
        .into_owned();
    generated(path)?
        .into_iter()
        .find(|(file, _)| *file == format!("{stem}.h"))
        .map(|(_, text)| text)
        .ok_or_else(|| format!("{stem} generates no `{stem}.h`"))
}

/// The C11 header of the resource `name`, which generates.
fn resource_header(name: &str) -> String {
    header_of(&resources().join(format!("{name}.scxml")))
        .unwrap_or_else(|e| panic!("{name} generates on C11: {e}"))
}

/// The forms of the resource `name` the C11 printer does not lower.
fn unlowered(name: &str) -> Vec<String> {
    c11_codec_unlowered_file(
        &resources().join(format!("{name}.scxml")),
        &[],
        &ForgeCompileOptions::default(),
    )
    .unwrap_or_else(|e| panic!("{name} reads: {e:?}"))
    .expect("the resource is a codec")
}

/// Whether `text` holds each of `needles`, with `text` in the failure.
fn holds(name: &str, text: &str, needles: &[&str]) {
    for needle in needles {
        assert!(
            text.contains(needle),
            "{name}: the header holds `{needle}`:\n{text}"
        );
    }
}

/// Whether `text` holds none of `needles`, with `text` in the failure.
fn holds_none(name: &str, text: &str, needles: &[&str]) {
    for needle in needles {
        assert!(
            !text.contains(needle),
            "{name}: the header holds no `{needle}`:\n{text}"
        );
    }
}

/// A codec of integers is presented by a table of a function for each element,
/// each handed the context; the encode reads each element once through the
/// table and writes it, and the value's encode is the encode over the value's
/// own table, so it reads no member of the value itself.
///
/// Turns red where `c11_source::render` stops naming a function of the table
/// for an element, where `encode_source` reads an element off the value rather
/// than through the table, or where the template's `encode` stops calling the
/// encode over the value's table.
#[test]
fn a_codec_of_integers_is_encoded_over_a_table_of_its_elements() {
    assert!(unlowered("codec_simple_frame").is_empty());
    let text = resource_header("codec_simple_frame");
    holds(
        "codec_simple_frame",
        &text,
        &[
            "typedef struct {\n    uint8_t (*msg_id)(const void *ctx);\n    \
             uint8_t (*length)(const void *ctx);\n    \
             uint16_t (*payload)(const void *ctx);\n} codec_simple_frame_source_t;\n",
            "static inline uint8_t codec_simple_frame_value_msg_id(const void *ctx) {\n    \
             return ((const codec_simple_frame_t *)ctx)->msg_id;\n}\n",
            "static inline const codec_simple_frame_source_t *codec_simple_frame_value_source(void) {\n    \
             static const codec_simple_frame_source_t table = {\n        \
             codec_simple_frame_value_msg_id,\n        \
             codec_simple_frame_value_length,\n        \
             codec_simple_frame_value_payload,\n    };\n    return &table;\n}\n",
            "static inline sce_forge_codec_status_t codec_simple_frame_encode_source(\
             const codec_simple_frame_source_t *src, const void *ctx, sce_forge_writer_t *w) {\n",
            "    const uint8_t _e_msg_id = src->msg_id(ctx);\n",
            "    const uint16_t _e_payload = src->payload(ctx);\n",
            "    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, _e_msg_id));\n",
            "    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, (uint8_t)((_e_payload >> 8) & 0xFF)));\n",
            "    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, (uint8_t)(_e_payload & 0xFF)));\n",
            "    return codec_simple_frame_encode_source(codec_simple_frame_value_source(), self, w);\n",
        ],
    );
    holds_none(
        "codec_simple_frame",
        &text,
        &["self->msg_id", "self->payload"],
    );
}

/// Fields that share a byte are written as the or of their pieces, each piece
/// read off the local the element was read into.
///
/// Turns red where `encode_source` writes a packed byte other than by
/// `encode_byte_read_by` over the elements' locals.
#[test]
fn fields_that_share_a_byte_are_written_from_the_elements_read_once() {
    assert!(unlowered("codec_subbyte").is_empty());
    let text = resource_header("codec_subbyte");
    holds(
        "codec_subbyte",
        &text,
        &["    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, (uint8_t)(((_e_priority & 0x07) << 5) \
           | ((_e_channel & 0x07) << 2) | ((_e_direction & 0x03) << 0))));\n"],
    );
}

/// A variable-length integer of a streamed layout is written by the runtime's
/// writer over the element read once.
///
/// Turns red where `encode_source` stops writing a VLE by `vle_encode_block`.
#[test]
fn a_variable_length_integer_is_written_by_the_runtimes_writer() {
    assert!(unlowered("codec_vle_zint_u64").is_empty());
    let text = resource_header("codec_vle_zint_u64");
    holds(
        "codec_vle_zint_u64",
        &text,
        &[
            "    const uint64_t _e_value = src->value(ctx);\n",
            "    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_vle_u64(w, (uint64_t)(_e_value)));\n",
        ],
    );
}

/// A codec whose plan uses a form the C11 printer does not lower keeps the
/// template's encode whole: a length the encode derives, a variant and a CBOR
/// map each name their form, and none of them has an encode over a source.
///
/// Turns red where `c11_source::positional_forms` stops naming a derived
/// element or a variant, or `unlowered_forms` a CBOR map, or where the template
/// prints the encode over a source for a codec it keeps.
#[test]
fn a_codec_of_a_form_the_printer_does_not_lower_keeps_the_template_encode() {
    for (name, form) in [
        ("codec_length_ref", "an element the encode derives"),
        ("codec_variant_default_marker", "a variant"),
        ("codec_cbor_map", "a CBOR map"),
    ] {
        let forms = unlowered(name);
        assert!(
            forms.iter().any(|f| f.contains(form)),
            "{name}: the printer names `{form}` among {forms:?}"
        );
        let text = resource_header(name);
        holds_none(name, &text, &["_encode_source(", "_value_source("]);
    }
}

/// Every codec of the resources is printed exactly where the printer lowers
/// each of its forms, and each printed one generates a header with its encode
/// over a source. The printed codecs are shown with `--nocapture`.
///
/// Turns red where `c11_codec_printed` and `c11_codec_unlowered` come apart,
/// where `render_codec` prints a codec the printer does not lower or leaves one
/// it lowers on the template's encode, or where the printer panics or the
/// template fails on a codec it lowers.
#[test]
fn the_c11_backend_prints_each_codec_of_the_resources_whose_forms_it_lowers() {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(resources())
        .expect("the resources directory")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "scxml"))
        .collect();
    paths.sort();
    let mut printed: Vec<String> = Vec::new();
    for path in &paths {
        let stem = path
            .file_stem()
            .expect("a document has a stem")
            .to_string_lossy()
            .into_owned();
        // A document that does not read, or is no codec, has no encode.
        let Ok(Some(forms)) = c11_codec_unlowered_file(path, &[], &ForgeCompileOptions::default())
        else {
            continue;
        };
        assert_eq!(
            c11_codec_printed_file(path, &[], &ForgeCompileOptions::default()).ok(),
            Some(Some(forms.is_empty())),
            "{stem}: printed exactly where no form is unlowered"
        );
        match header_of(path) {
            Ok(text) => assert_eq!(
                text.contains("_encode_source(const "),
                forms.is_empty(),
                "{stem}: the header prints its encode over a source exactly where the \
                 printer lowers every form ({forms:?})"
            ),
            // A codec C11 refuses to hold generates on no tree; one the printer
            // prints generates.
            Err(refusal) => assert!(
                !forms.is_empty(),
                "{stem}: a codec the printer prints generates on C11: {refusal}"
            ),
        }
        if forms.is_empty() {
            printed.push(stem);
        }
    }
    eprintln!("{} codec(s) printed on C11:", printed.len());
    for stem in &printed {
        eprintln!("  {stem}");
    }
    for name in [
        "codec_simple_frame",
        "codec_flags_basic",
        "codec_subbyte",
        "codec_vle_zint_u64",
    ] {
        assert!(
            printed.iter().any(|stem| stem == name),
            "{name} is printed on C11"
        );
    }
}

/// A consumer's own record, of a layout of its own, is a source of the codec by
/// a table of its own, and its encode writes the frame the codec's value writes.
const DRIVER: &str = r#"
#include <stdio.h>
#include <string.h>

#include "codec_simple_frame.h"

/* A record the codec does not know, holding the elements in its own types. */
struct reading {
    unsigned id;
    unsigned size;
    unsigned long value;
};

static uint8_t reading_msg_id(const void *ctx) {
    return (uint8_t)((const struct reading *)ctx)->id;
}

static uint8_t reading_length(const void *ctx) {
    return (uint8_t)((const struct reading *)ctx)->size;
}

static uint16_t reading_payload(const void *ctx) {
    return (uint16_t)((const struct reading *)ctx)->value;
}

static const codec_simple_frame_source_t reading_source = {
    reading_msg_id,
    reading_length,
    reading_payload,
};

static int failures = 0;

#define CHECK(cond, what)                                    \
    do {                                                     \
        if (!(cond)) {                                       \
            fprintf(stderr, "FAIL: %s\n", what);             \
            ++failures;                                      \
        }                                                    \
    } while (0)

int main(void) {
    static const uint8_t frame[] = {0x01, 0x02, 0x03, 0x04};
    const struct reading r = {0x01u, 0x02u, 0x0304ul};

    uint8_t by_source[CODEC_SIMPLE_FRAME_MAX_BYTES];
    sce_forge_writer_t w = sce_forge_writer_init_buf(by_source, sizeof by_source);
    CHECK(codec_simple_frame_encode_source(&reading_source, &r, &w) == SCE_FORGE_CODEC_OK,
          "the record encodes");
    CHECK(w.pos == sizeof frame && memcmp(by_source, frame, sizeof frame) == 0,
          "the record writes the frame");

    codec_simple_frame_t v;
    memset(&v, 0, sizeof v);
    v.msg_id = 0x01u;
    v.length = 0x02u;
    v.payload = 0x0304u;
    uint8_t by_value[CODEC_SIMPLE_FRAME_MAX_BYTES];
    size_t n = 0;
    CHECK(codec_simple_frame_encode_to_buf(&v, by_value, sizeof by_value, &n) == SCE_FORGE_CODEC_OK,
          "the value encodes");
    CHECK(n == sizeof frame && memcmp(by_value, frame, n) == 0, "the value writes the frame");

    /* The sink's room is the one thing the encode refuses on. */
    uint8_t tiny[1];
    sce_forge_writer_t t = sce_forge_writer_init_buf(tiny, sizeof tiny);
    CHECK(codec_simple_frame_encode_source(&reading_source, &r, &t) == SCE_FORGE_CODEC_BUFFER_OVERFLOW,
          "a writer with no room left is refused");

    return failures == 0 ? 0 : 1;
}
"#;

/// A program that presents a record of its own through a table of its own
/// compiles against the printed header as C11 with the strictest warnings, and
/// its encode writes the frame the codec's value writes.
///
/// Turns red where the printed table, the value's table or the encode over a
/// source stops compiling as C11, or where the encode over a source writes
/// other than the value's encode.
#[test]
fn a_record_of_its_own_is_encoded_through_a_table_of_its_own() {
    let dir = tempdir().expect("tempdir");
    for (file, text) in generated(&resources().join("codec_simple_frame.scxml"))
        .expect("codec_simple_frame generates on C11")
    {
        std::fs::write(dir.path().join(file), text).expect("write a generated file");
    }
    let driver = dir.path().join("driver.c");
    std::fs::write(&driver, DRIVER).expect("write the driver");
    let binary = dir.path().join("driver");
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
    let backends_c = Path::new(env!("CARGO_MANIFEST_DIR")).join("../backends/c");
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
        "the driver compiles against the printed header:\n{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let run = Command::new(&binary).output().expect("the driver runs");
    assert!(
        run.status.success(),
        "the driver's checks hold:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
}
