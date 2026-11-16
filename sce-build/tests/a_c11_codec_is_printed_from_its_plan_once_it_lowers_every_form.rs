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

/// A length is derived from the bytes of the view it measures and is no member
/// of the value: the decode reads it into a local and cuts the bytes by it. The
/// byte string is presented as a view of a type of its own, made by one
/// function that refuses a view outside the bound, and the value's encode
/// holds the array to that bound before the first byte.
///
/// Turns red where `c11_source::encode_source` reads the length off the source
/// rather than measuring the view, where the template's struct keeps a member
/// the encode derives, where the decode reads a derived length off the struct,
/// or where `BoundedView::declare` stops checking the bound.
#[test]
fn a_length_is_derived_from_the_view_it_measures_and_is_no_member_of_the_value() {
    assert!(unlowered("codec_length_ref").is_empty());
    let text = resource_header("codec_length_ref");
    holds(
        "codec_length_ref",
        &text,
        &[
            "typedef struct {\n    const uint8_t *data;\n    size_t len;\n} \
             codec_length_ref_payload_bytes_t;\n",
            "static inline sce_forge_codec_status_t codec_length_ref_payload_bytes_make(\
             codec_length_ref_payload_bytes_t *out, const uint8_t *data, size_t len) {\n    \
             if (!sce_forge_length_within_bound(len, 32)) {\n        \
             return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;\n    }\n",
            "    codec_length_ref_payload_bytes_t (*payload)(const void *ctx);\n",
            "    view.data = self->payload;\n    view.len = self->payload_len;\n",
            "    const codec_length_ref_payload_bytes_t _e_payload = src->payload(ctx);\n",
            "    const uint8_t _d_len = (uint8_t)(_e_payload.len);\n",
            "    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, _d_len));\n",
            "    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_bytes(w, _e_payload.data, _e_payload.len));\n",
            "    const uint8_t _d_len = raw[1];\n",
            "size_t _n = (size_t)_d_len;",
            "    if (!sce_forge_length_within_bound(self->payload_len, 32)) {\n        \
             return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;\n    }\n    \
             return codec_length_ref_encode_source(codec_length_ref_value_source(), self, w);\n",
        ],
    );
    holds_none(
        "codec_length_ref",
        &text,
        &["    uint8_t len;\n", "out->len", "self->len", "src->len"],
    );
}

/// A length its `sce:length-arith` takes off is derived less that, and the view
/// of the bytes holds at least what it takes off, which its maker and the
/// value's encode both check, so the number is never below zero.
///
/// Turns red where `c11_source::encode_source` drops the `Offset::Less`
/// arithmetic, or where the maker or the value's encode stops checking the
/// least the bound holds.
#[test]
fn a_length_less_its_arithmetic_is_derived_less_it() {
    assert!(unlowered("codec_scout_zid_body").is_empty());
    let text = resource_header("codec_scout_zid_body");
    holds(
        "codec_scout_zid_body",
        &text,
        &[
            "    if (len < 1u || !sce_forge_length_within_bound(len, 16)) {\n",
            "    const uint8_t _d_zid_len_m1 = (uint8_t)(_e_zid.len - 1u);\n",
            "    if (self->zid_len < 1u || !sce_forge_length_within_bound(self->zid_len, 16)) {\n",
            "(size_t)((int64_t)_d_zid_len_m1 + 1)",
        ],
    );
    holds_none(
        "codec_scout_zid_body",
        &text,
        &["    uint8_t zid_len_m1;\n", "self->zid_len_m1"],
    );
}

/// A length whose field is named as the array's own length member leaves the
/// value as any derived length does, and the array keeps a length member of
/// its own, which the decode sets from the bytes it cut.
///
/// Turns red where `render_codec` keeps suppressing the array's length member
/// for a sibling the encode derives.
#[test]
fn a_length_named_as_the_arrays_length_member_leaves_the_value_too() {
    assert!(unlowered("codec_length_ref_uint16_be").is_empty());
    let text = resource_header("codec_length_ref_uint16_be");
    holds(
        "codec_length_ref_uint16_be",
        &text,
        &[
            "    uint8_t payload[1024];\n    size_t  payload_len;\n",
            "    const uint16_t _d_payload_len = (uint16_t)(_e_payload.len);\n",
            "        out->payload_len = _n;\n",
        ],
    );
    holds_none(
        "codec_length_ref_uint16_be",
        &text,
        &["    uint16_t payload_len;\n"],
    );
}

/// A text of a streamed layout whose length is a variable-length integer before
/// it: the encode derives the number from the view and writes it by the
/// runtime's writer, and the decode reads it into a local of its own that the
/// text's length is read off, as the value holds no member of it. The text's
/// array keeps the length member of its own the number was named as.
///
/// Turns red where `present_if_decode_vle` reads a derived number into the
/// struct, where `carried_length` reads the length off the struct, or where
/// `c11_source::encode_source` writes no byte string of a streamed layout.
#[test]
fn a_streamed_length_is_derived_and_read_into_a_local_of_the_decode() {
    assert!(unlowered("codec_zenoh_locator").is_empty());
    let text = resource_header("codec_zenoh_locator");
    holds(
        "codec_zenoh_locator",
        &text,
        &[
            "    codec_zenoh_locator_locator_text_t (*locator)(const void *ctx);\n",
            "    size_t  locator_len;\n",
            "    const uint64_t _d_locator_len = (uint64_t)(_e_locator.len);\n",
            "sce_forge_writer_write_vle_u64(w, (uint64_t)(_d_locator_len))",
            "    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_bytes(w, \
             (const uint8_t *)_e_locator.data, _e_locator.len));\n",
            "uint64_t _d_locator_len;\n",
            "sce_forge_cursor_read_vle_u64(cursor, &_d_locator_len)",
            "size_t _n = (size_t)_d_locator_len;",
        ],
    );
    holds_none(
        "codec_zenoh_locator",
        &text,
        &[
            "    uint64_t locator_len;\n",
            "out->locator_len = locator_len;",
            "src->locator_len",
        ],
    );
}

/// A fixed field after a byte string of a streamed layout, whose length is a
/// fixed field before it: the decode declares the length's local before the
/// block that reads it, and the encode writes the bytes before the field after
/// them, as the frame lays them out.
///
/// Turns red where `present_if_decode_fixed` declares the local inside its
/// block or writes the struct, or where the encode over a source writes the
/// streamed fields out of their order.
#[test]
fn a_fixed_length_of_a_streamed_layout_is_read_into_a_local_before_its_block() {
    assert!(unlowered("codec_fixed_after_lengthref").is_empty());
    let text = resource_header("codec_fixed_after_lengthref");
    holds(
        "codec_fixed_after_lengthref",
        &text,
        &[
            "uint16_t _d_payload_len;\n    {\n",
            "        _d_payload_len = ",
            "    const uint16_t _d_payload_len = (uint16_t)(_e_payload.len);\n",
            "size_t _n = (size_t)_d_payload_len;",
        ],
    );
    holds_none(
        "codec_fixed_after_lengthref",
        &text,
        &[
            "    uint16_t payload_len;\n",
            "out->payload_len = (uint16_t)",
        ],
    );
    let bytes = text
        .find("sce_forge_writer_write_bytes(w, _e_payload.data, _e_payload.len)")
        .expect("the encode writes the payload's view");
    let crc = text[bytes..].find("_e_crc32");
    assert!(
        crc.is_some(),
        "the encode writes crc32 after the payload it follows"
    );
}

/// A text a gate decides, with its length gated beside it: the source
/// presents the text as the optional value of its view, the encode folds the
/// presence flag into the carrier from whether the text is there, derives the
/// length as zero where it is not, writes the length where the gate holds over
/// the carrier it writes and the text where the source holds it, and the
/// decode reads the length into a local that is zero where the gate is clear.
///
/// Turns red where `c11_source::encode_source` stops folding a presence flag,
/// derives a gated length off an absent view, writes a gated field
/// unguarded, or where `present_if_decode_fixed` writes a derived length into
/// the struct.
#[test]
fn a_gated_text_sets_its_flag_and_its_length_from_whether_it_is_there() {
    assert!(unlowered("codec_present_if_string").is_empty());
    let text = resource_header("codec_present_if_string");
    holds(
        "codec_present_if_string",
        &text,
        &[
            "SCE_OPTIONAL_DECLARE_NAMED(codec_present_if_string_text_text_optional, \
             codec_present_if_string_text_text_t)\n",
            "    codec_present_if_string_text_text_optional_t (*text)(const void *ctx);\n",
            "        return codec_present_if_string_text_text_optional_none();\n",
            "    view.data = (const char *)self->text.value.data;\n",
            "    _f_carrier = (_f_carrier & ~0x1ull) | (uint64_t)(_e_text.present);\n",
            "    const uint8_t _c_carrier = (uint8_t)_f_carrier;\n",
            "    const uint8_t _d_text_len = _e_text.present ? (uint8_t)(_e_text.value.len) : \
             (uint8_t)0;\n",
            "    if ((((uint64_t)_c_carrier & 0x1ull) != 0)) {\n",
            "    if (_e_text.present) {\n        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_bytes(w, \
             (const uint8_t *)_e_text.value.data, _e_text.value.len));\n    }\n",
            "uint8_t _d_text_len = 0;\n    if (",
            "    if (self->text.present && (!sce_forge_length_within_bound(self->text.value.len, 32))) {\n",
        ],
    );
    holds_none(
        "codec_present_if_string",
        &text,
        &[" text_len;\n", "self->text_len", "out->text_len"],
    );
}

/// A codec whose plan uses a form the C11 printer does not lower keeps the
/// template's encode whole: a variant and a CBOR map each name their form, and
/// neither has an encode over a source.
///
/// Turns red where `c11_source::positional_forms` stops naming a variant, or
/// `unlowered_forms` a CBOR map, or where the template prints the encode over
/// a source for a codec it keeps.
#[test]
fn a_codec_of_a_form_the_printer_does_not_lower_keeps_the_template_encode() {
    for (name, form) in [
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
        "codec_length_ref",
        "codec_length_ref_uint16_be",
        "codec_scout_zid_body",
        "codec_tail",
        "codec_zenoh_locator",
        "codec_fixed_after_lengthref",
        "codec_zenoh_frame",
        "codec_present_if_string",
        "codec_present_if_tail",
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

#include "codec_length_ref.h"
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

/* A message of its own, its payload a view made once by the codec's maker. */
struct message {
    unsigned id;
    codec_length_ref_payload_bytes_t payload;
};

static uint8_t message_msg_id(const void *ctx) {
    return (uint8_t)((const struct message *)ctx)->id;
}

static codec_length_ref_payload_bytes_t message_payload(const void *ctx) {
    return ((const struct message *)ctx)->payload;
}

static const codec_length_ref_source_t message_source = {
    message_msg_id,
    message_payload,
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

    /* A byte string is a view its maker holds to the bound; the length is
     * derived from the view, and the value holds none. */
    static const uint8_t bytes[33] = {0xAA, 0xBB, 0xCC};
    struct message msg;
    msg.id = 0x09u;
    CHECK(codec_length_ref_payload_bytes_make(&msg.payload, bytes, 33) == SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE,
          "a view past the bound is refused");
    CHECK(codec_length_ref_payload_bytes_make(&msg.payload, bytes, 3) == SCE_FORGE_CODEC_OK, "a view within it");
    static const uint8_t with_length[] = {0x09, 0x03, 0xAA, 0xBB, 0xCC};
    uint8_t by_message[CODEC_LENGTH_REF_MAX_BYTES];
    sce_forge_writer_t mw = sce_forge_writer_init_buf(by_message, sizeof by_message);
    CHECK(codec_length_ref_encode_source(&message_source, &msg, &mw) == SCE_FORGE_CODEC_OK, "the message encodes");
    CHECK(mw.pos == sizeof with_length && memcmp(by_message, with_length, sizeof with_length) == 0,
          "the message writes the length of its view");

    static codec_length_ref_t lv;
    memset(&lv, 0, sizeof lv);
    lv.msg_id = 0x09u;
    memcpy(lv.payload, bytes, 3);
    lv.payload_len = 3;
    uint8_t by_lvalue[CODEC_LENGTH_REF_MAX_BYTES];
    size_t ln = 0;
    CHECK(codec_length_ref_encode_to_buf(&lv, by_lvalue, sizeof by_lvalue, &ln) == SCE_FORGE_CODEC_OK &&
              ln == sizeof with_length && memcmp(by_lvalue, with_length, ln) == 0,
          "the value writes the length of its bytes");
    static codec_length_ref_t back;
    memset(&back, 0, sizeof back);
    sce_forge_cursor_t c = sce_forge_cursor_init(with_length, sizeof with_length);
    CHECK(codec_length_ref_decode(&c, &back) == SCE_FORGE_CODEC_OK && back.payload_len == 3 &&
              memcmp(back.payload, bytes, 3) == 0,
          "the decode cuts the bytes by the length it read");
    lv.payload_len = sizeof lv.payload + 1;
    CHECK(codec_length_ref_encode_to_buf(&lv, by_lvalue, sizeof by_lvalue, &ln) ==
              SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE,
          "a value past its array's bound is no value of the codec");

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
    for name in ["codec_simple_frame", "codec_length_ref"] {
        for (file, text) in generated(&resources().join(format!("{name}.scxml")))
            .unwrap_or_else(|e| panic!("{name} generates on C11: {e}"))
        {
            std::fs::write(dir.path().join(file), text).expect("write a generated file");
        }
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
