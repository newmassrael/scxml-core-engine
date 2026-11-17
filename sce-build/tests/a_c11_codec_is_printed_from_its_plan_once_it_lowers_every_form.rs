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
            "static inline sce_forge_codec_status_t codec_length_ref_check_value(\
             const codec_length_ref_t *self) {\n    \
             if (!sce_forge_length_within_bound(self->payload_len, 32)) {\n        \
             return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;\n    }\n    \
             return SCE_FORGE_CODEC_OK;\n}\n",
            "    SCE_FORGE_TRY_WRITE(codec_length_ref_check_value(self));\n    \
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

/// A body of a codec the C11 backend prints is presented as the table and the
/// context of its codec's source, which the value's reader points at the body
/// it holds, and written by its codec's encode over that source; the value's
/// check holds the body's arrays to their bounds through its codec's check, as
/// the encode over a source reads them through its views.
///
/// Turns red where `c11_source::positional_forms` refuses a body of a printed
/// codec, where the encode over a source writes a body other than through its
/// codec's `encode_source`, or where `check_value` stops checking a body.
#[test]
fn a_body_is_presented_as_its_codecs_source_and_written_by_its_encode() {
    assert!(unlowered("codec_embed_basic").is_empty());
    let text = resource_header("codec_embed_basic");
    holds(
        "codec_embed_basic",
        &text,
        &[
            "#include \"sce/forge/optional.h\"",
            "typedef struct {\n    const codec_embed_basic_source_t *table;\n    \
             const void *ctx;\n} codec_embed_basic_source_ref_t;\n",
            "SCE_OPTIONAL_DECLARE_NAMED(codec_embed_basic_source_ref_optional, \
             codec_embed_basic_source_ref_t)\n",
            "    codec_zenoh_locator_source_ref_t (*locator)(const void *ctx);\n",
            "    body.table = codec_zenoh_locator_value_source();\n    \
             body.ctx = &self->locator;\n",
            "    SCE_FORGE_TRY_WRITE(codec_zenoh_locator_encode_source(_e_locator.table, \
             _e_locator.ctx, w));\n",
            "    SCE_FORGE_TRY_WRITE(codec_zenoh_locator_check_value(&self->locator));\n",
        ],
    );
}

/// The encoded length of a body is what its codec's encode over the source the
/// value presents writes into the runtime's counting sink, which stores
/// nothing; the value holds no member of it, and the decode bounds the body by
/// the local it read the length into.
///
/// Turns red where `c11_source::encode_source` takes the length off the value
/// rather than counting the body, or where `embed_streaming_decode_stmt` reads
/// a derived length off the struct.
#[test]
fn an_encoded_length_is_counted_by_the_bodys_encode_into_a_counting_sink() {
    assert!(unlowered("codec_zenoh_timestamp_ext").is_empty());
    let text = resource_header("codec_zenoh_timestamp_ext");
    holds(
        "codec_zenoh_timestamp_ext",
        &text,
        &[
            "    sce_forge_writer_t _k_ts = sce_forge_writer_init_counting();\n    \
             SCE_FORGE_TRY_WRITE(codec_zenoh_timestamp_encode_source(_e_ts.table, _e_ts.ctx, \
             &_k_ts));\n    \
             const uint64_t _d_ext_size = (uint64_t)(sce_forge_writer_position(&_k_ts));\n",
            "uint64_t _d_ext_size;\n",
        ],
    );
    holds_none(
        "codec_zenoh_timestamp_ext",
        &text,
        &[
            "out->ext_size",
            "self->ext_size",
            "    uint64_t ext_size;\n",
        ],
    );
}

/// Two byte strings always there share one length: the first says it, and the
/// encode refuses the second where it says another, before the first byte, by
/// the runtime's status of an inconsistent source.
///
/// Turns red where `c11_source::derive_number` stops holding a second measure
/// always there to the first, or where `positional_forms` names a number of
/// two measures as unlowered again.
#[test]
fn a_length_two_byte_strings_share_is_the_first_and_the_second_is_held_to_it() {
    assert!(unlowered("codec_length_shared").is_empty());
    let text = resource_header("codec_length_shared");
    holds(
        "codec_length_shared",
        &text,
        &[
            "    if ((size_t)(_e_beta.len) != (size_t)(_e_alpha.len)) {\n        \
             return SCE_FORGE_CODEC_INCONSISTENT_SOURCE;\n    }\n    \
             const uint8_t _d_span_len = (uint8_t)(_e_alpha.len);\n",
            "A value the encoding cannot carry is refused",
        ],
    );
    holds_none(
        "codec_length_shared",
        &text,
        &["    uint8_t span_len;\n", "It refuses no value"],
    );
}

/// A length of two gated byte strings the value states where neither is
/// there: the source presents it as an optional number, the encode derives it
/// from the first byte string there, holds the other to it and holds what the
/// value states beside content to what the content says, and the decode
/// answers it present exactly where neither byte string was there.
///
/// Turns red where `c11_source::derive_number` takes a stated number where
/// content is there or stops refusing one that disagrees, where the template's
/// struct stops holding a stated number as an optional value, or where
/// `c11_source::decoded` stops setting it from the local the decode read.
#[test]
fn a_length_the_value_states_is_derived_from_the_first_content_there() {
    assert!(unlowered("codec_length_stated").is_empty());
    let text = resource_header("codec_length_stated");
    holds(
        "codec_length_stated",
        &text,
        &[
            "    sce_optional_uint8_t (*span_len)(const void *ctx);\n",
            "    sce_optional_uint8_t span_len;\n",
            "    size_t _m_span_len = 0;\n    bool _m_span_len_there = false;\n    \
             if (_e_alpha.present) {\n        _m_span_len = (size_t)(_e_alpha.value.len);\n        \
             _m_span_len_there = true;\n    } else if (_e_beta.present) {\n        \
             _m_span_len = (size_t)(_e_beta.value.len);\n        _m_span_len_there = true;\n    \
             }\n    if (_e_beta.present && (size_t)(_e_beta.value.len) != _m_span_len) {\n        \
             return SCE_FORGE_CODEC_INCONSISTENT_SOURCE;\n    }\n",
            "    const uint8_t _d_span_len = _m_span_len_there ? (uint8_t)(_m_span_len) : \
             (_e_span_len.present ? _e_span_len.value : (uint8_t)0);\n    \
             if (_m_span_len_there && _e_span_len.present && _e_span_len.value != _d_span_len) {\n        \
             return SCE_FORGE_CODEC_INCONSISTENT_SOURCE;\n    }\n",
            "    out->span_len = (!out->alpha.present && !out->beta.present) ? \
             sce_optional_uint8_some(_d_span_len) : sce_optional_uint8_none();",
        ],
    );
}

/// The encoded length of a body whose own encode can refuse: the count passes
/// the body's refusal on, a length past the field is refused, and the encode
/// over a source writes the frame over a counting sink first, so a refusal is
/// met before the first byte.
///
/// Turns red where `c11_source::encode_source` writes without counting first
/// for a codec that holds a body that can refuse, or where `derive_number`
/// stops refusing a number that may not fit its field.
#[test]
fn a_body_that_can_refuse_is_counted_before_the_first_byte() {
    assert!(unlowered("codec_encoded_length_checked").is_empty());
    let text = resource_header("codec_encoded_length_checked");
    holds(
        "codec_encoded_length_checked",
        &text,
        &[
            "static inline sce_forge_codec_status_t codec_encoded_length_checked_write_source(\
             const codec_encoded_length_checked_source_t *src, const void *ctx, \
             sce_forge_writer_t *w) {\n",
            "    if (!sce_forge_length_within_bound((size_t)(sce_forge_writer_position(&_k_inner)), \
             65535)) {\n        return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;\n    }\n    \
             const uint16_t _d_inner_size = (uint16_t)(sce_forge_writer_position(&_k_inner));\n",
            "        sce_forge_writer_t _count = sce_forge_writer_init_counting();\n        \
             SCE_FORGE_TRY_WRITE(codec_encoded_length_checked_write_source(src, ctx, &_count));\n    \
             }\n    return codec_encoded_length_checked_write_source(src, ctx, w);\n",
            "    SCE_FORGE_TRY_WRITE(codec_length_shared_check_value(&self->inner));\n",
        ],
    );
}

/// A body a gate decides, of a codec the C11 backend prints, is a form the
/// printer lowers: a codec that holds one beside a list of printed entries is
/// printed whole, and one that holds one beside a chain of entries the C11
/// backend does not print names no form of the body and waits on the chain.
///
/// Turns red where `c11_source::positional_forms` names a gated body or a list
/// of a printed codec, or where `codec_zenoh_msg_del` stops holding a chain of
/// unprinted entries, which would leave the census with nothing to say.
#[test]
fn a_gated_body_of_a_printed_codec_is_no_form_the_printer_leaves() {
    assert!(unlowered("codec_origin_envelope").is_empty());
    let forms = unlowered("codec_zenoh_msg_del");
    assert!(
        !forms.iter().any(|f| f.starts_with("field 'timestamp'")),
        "codec_zenoh_msg_del: the gated body `timestamp` is lowered ({forms:?})"
    );
    assert!(
        forms.iter().any(|f| f.contains("a list of")),
        "codec_zenoh_msg_del: the codec waits on its chain ({forms:?})"
    );
}

/// A list is presented as a walk of its entries, made by a maker that counts
/// them within the bound: the encode derives the count from what the maker
/// counted, walks the entries from the first and writes each by its codec's
/// encode over the source the list presents it as; the value holds no member
/// of the count, its walk reads the array the value holds, its check holds
/// each entry to its codec, and the decode reads the count into a local.
///
/// Turns red where `BoundedList::declare` stops declaring the list or its
/// maker, where `measure_text` reads a count off anything but the list, where
/// `encode_source` stops walking the entries, where `check_value` stops
/// checking each entry, or where `repeat_streaming_decode_stmt` reads a
/// derived count off the struct.
#[test]
fn a_list_is_presented_as_a_walk_and_its_count_derived_from_its_maker() {
    assert!(unlowered("codec_repeat_basic").is_empty());
    let text = resource_header("codec_repeat_basic");
    holds(
        "codec_repeat_basic",
        &text,
        &[
            "typedef struct {\n    const sce_forge_walk_t *walk;\n    const void *ctx;\n    \
             const codec_repeat_elem_source_t *entry;\n    size_t count;\n\
             } codec_repeat_basic_frags_list_t;\n",
            "static inline sce_forge_codec_status_t codec_repeat_basic_frags_list_make(\
             codec_repeat_basic_frags_list_t *out, const sce_forge_walk_t *walk, \
             const void *ctx, const codec_repeat_elem_source_t *entry) {\n",
            "        if (!sce_forge_length_within_bound(count, 32)) {\n            \
             return SCE_FORGE_CODEC_TOO_MANY_ELEMENTS;\n        }\n",
            "    codec_repeat_basic_frags_list_t (*frags)(const void *ctx);\n",
            "    return self->frags_len > 0u ? (const void *)&self->frags[0] : NULL;\n",
            "    list.entry = codec_repeat_elem_value_source();\n    \
             list.count = self->frags_len;\n",
            "    const uint8_t _d_num_frags = (uint8_t)(_e_frags.count);\n",
            "        const void *_at = _e_frags.walk->begin(_e_frags.ctx);\n        \
             for (size_t _i = 0; _i < _e_frags.count && _at != NULL; ++_i) {\n            \
             SCE_FORGE_TRY_WRITE(codec_repeat_elem_encode_source(_e_frags.entry, _at, w));\n            \
             _at = _e_frags.walk->next(_e_frags.ctx, _at);\n        }\n",
            "    for (size_t _i = 0; _i < self->frags_len; ++_i) {\n        \
             SCE_FORGE_TRY_WRITE(codec_repeat_elem_check_value(&self->frags[_i]));\n    }\n",
            "size_t _n = (size_t)_d_num_frags;",
        ],
    );
    holds_none(
        "codec_repeat_basic",
        &text,
        &[
            "    uint8_t num_frags;\n",
            "out->num_frags",
            "self->num_frags",
        ],
    );
}

/// A list a gate decides is presented as the optional value of its list: the
/// flag that gates it is set from whether the source holds it, a count gated
/// beside it is the count its maker found where it is there and zero where it
/// is not, and its entries are written and checked only where it is there.
///
/// Turns red where `presented_type` presents a gated list as other than its
/// optional value, where the value's reader stops answering it absent where
/// the struct holds none, or where `check_value` checks a list that is absent.
#[test]
fn a_gated_list_is_presented_as_an_optional_list() {
    assert!(unlowered("codec_repeat_present_if_basic").is_empty());
    let text = resource_header("codec_repeat_present_if_basic");
    holds(
        "codec_repeat_present_if_basic",
        &text,
        &[
            "SCE_OPTIONAL_DECLARE_NAMED(codec_repeat_present_if_basic_elems_list_optional, \
             codec_repeat_present_if_basic_elems_list_t)\n",
            "    codec_repeat_present_if_basic_elems_list_optional_t (*elems)(const void *ctx);\n",
            "    if (!self->elems.present) {\n        \
             return codec_repeat_present_if_basic_elems_list_optional_none();\n    }\n",
            "    const uint8_t _d_num_elems = _e_elems.present ? (uint8_t)(_e_elems.value.count) \
             : (uint8_t)0;\n",
            "    if (_e_elems.present) {\n",
            "    if (self->elems.present) {\n",
        ],
    );
}

/// A chain that ends with its scope is a list as any other: walked entry by
/// entry, with no count before it.
///
/// Turns red where `positional_forms` names a chain that sets no flag of its
/// entries, or where `encode_source` writes a chain other than as a walk.
#[test]
fn a_chain_that_ends_with_its_scope_is_walked_as_a_list() {
    assert!(unlowered("codec_tlv_chain_basic").is_empty());
    let text = resource_header("codec_tlv_chain_basic");
    holds(
        "codec_tlv_chain_basic",
        &text,
        &[
            "    codec_tlv_chain_basic_extensions_list_t (*extensions)(const void *ctx);\n",
            "SCE_FORGE_TRY_WRITE(codec_tlv_entry_encode_source(_e_extensions.entry, _at, w));\n",
        ],
    );
}

/// A one-bit flag a value presents is one a parent may set in place of the
/// value's: the codec declares its overrides, folds a flag set there into its
/// carrier last, and its plain encode sets none. A chain closed by an entry
/// flag sets that flag on each entry but the last, and holds one entry or
/// more.
///
/// Turns red where `positional_forms` names an overridable flag or a chain
/// that sets its entries' flag, where `encode_source` stops folding an
/// override, where the chain's walk stops setting the flag from the entry's
/// place, or where the list's maker admits an empty chain closed by a flag.
#[test]
fn a_chain_closed_by_an_entry_flag_sets_it_on_each_entry_but_the_last() {
    assert!(unlowered("codec_chain_depth_entry").is_empty());
    let entry = resource_header("codec_chain_depth_entry");
    holds(
        "codec_chain_depth_entry",
        &entry,
        &[
            "typedef struct {\n    /* The `Z` flag of `header`, where a parent sets it. */\n    \
             sce_optional_bool_t header_z;\n} codec_chain_depth_entry_overrides_t;\n",
            "    if (overrides.header_z.present) {\n        \
             _f_header = (_f_header & ~0x80ull) | ((uint64_t)overrides.header_z.value << 7);\n    \
             }\n",
            "    return codec_chain_depth_entry_encode_source_overriding(src, ctx, w, \
             codec_chain_depth_entry_overrides_none());\n",
        ],
    );
    assert!(unlowered("codec_chain_depth_plain").is_empty());
    let chain = resource_header("codec_chain_depth_plain");
    holds(
        "codec_chain_depth_plain",
        &chain,
        &[
            "    if (count < 1u) {\n        return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;\n    }\n",
            "            codec_chain_depth_entry_overrides_t _o = \
             codec_chain_depth_entry_overrides_none();\n            \
             _o.header_z = sce_optional_bool_some(_i + 1u < _e_extensions.count);\n            \
             SCE_FORGE_TRY_WRITE(codec_chain_depth_entry_encode_source_overriding(\
             _e_extensions.entry, _at, w, _o));\n",
        ],
    );
}

/// A flag input the value says by the one field gated on it is read off the
/// source, not handed to the encode (docs/adr/0065 §6 R4a): the encode reads it
/// off whether the field is there, a length gated on it is written where that
/// holds, and the input's own function answers it for a parent. The value's
/// `encode` takes no argument for it; the decode still takes it.
///
/// Turns red where `positional_forms` names a flag input again, where
/// `encode_source` takes a said input as a parameter or stops reading it off
/// the element, where `gate_test` stops testing an input clause, or where
/// `said_inputs` stops printing the input's function.
#[test]
fn an_input_the_value_says_is_read_off_the_field_gated_on_it() {
    assert!(unlowered("codec_zenoh_wireexpr").is_empty());
    let text = resource_header("codec_zenoh_wireexpr");
    holds(
        "codec_zenoh_wireexpr",
        &text,
        &[
            "static inline sce_forge_codec_status_t codec_zenoh_wireexpr_encode_source(\
             const codec_zenoh_wireexpr_source_t *src, const void *ctx, sce_forge_writer_t *w) {\n",
            "    const uint8_t n = (uint8_t)(_e_suffix.present ? 1u : 0u);\n",
            "(((uint64_t)n & 0x1ull) != 0)",
            "static inline uint8_t codec_zenoh_wireexpr_input_n(\
             const codec_zenoh_wireexpr_source_t *src, const void *ctx) {\n    \
             return (uint8_t)(src->suffix(ctx).present ? 1u : 0u);\n}\n",
            "static inline sce_forge_codec_status_t codec_zenoh_wireexpr_encode(\
             const codec_zenoh_wireexpr_t *self, sce_forge_writer_t *w) {\n",
            "    return codec_zenoh_wireexpr_encode_source(codec_zenoh_wireexpr_value_source(), \
             self, w);\n",
        ],
    );
    holds_none(
        "codec_zenoh_wireexpr",
        &text,
        &["sce_forge_writer_t *w, uint8_t n)"],
    );
}

/// An input a body says is said by the codec that holds the body and hands it
/// on: the holder's own function of it reads the body's, and the body is
/// written handed nothing for it.
///
/// Turns red where `said_input` stops reading a forwarded input through the
/// body's codec's function, or where `handed_args` hands a body an input it
/// says.
#[test]
fn an_input_a_body_says_is_read_through_the_bodys_codec() {
    assert!(unlowered("codec_zenoh_decl_kexpr").is_empty());
    let text = resource_header("codec_zenoh_decl_kexpr");
    holds(
        "codec_zenoh_decl_kexpr",
        &text,
        &[
            "static inline uint8_t codec_zenoh_decl_kexpr_input_n(\
             const codec_zenoh_decl_kexpr_source_t *src, const void *ctx) {\n    \
             return codec_zenoh_wireexpr_input_n(src->wireexpr(ctx).table, \
             src->wireexpr(ctx).ctx);\n}\n",
            "    SCE_FORGE_TRY_WRITE(codec_zenoh_wireexpr_encode_source(_e_wireexpr.table, \
             _e_wireexpr.ctx, w));\n",
        ],
    );
}

/// A variant whose tag is a field of its own: the source presents the arm it
/// holds as its kind beside the source of the arm's body, the encode derives
/// the tag from that kind, a known arm's the document's and the catch-all's
/// the one it holds, and writes the arm by its codec's encode over the arm's
/// body. The value holds no member of the tag, which the decode reads into a
/// local and dispatches on, and its check holds the arm's body to its codec.
///
/// Turns red where `c11_source::positional_forms` names a variant again,
/// where `declare_body_ref` stops holding the arm's source in the member its
/// kind names, where `encode_source` reads the tag off anything but the arm
/// or writes the arm other than by its codec's encode, where `check_value`
/// stops checking the arm, or where `render_codec` dispatches the decode on a
/// member the value no longer holds.
#[test]
fn a_variant_derives_its_tag_from_the_arm_a_source_presents() {
    assert!(unlowered("codec_variant_dispatch").is_empty());
    let text = resource_header("codec_variant_dispatch");
    holds(
        "codec_variant_dispatch",
        &text,
        &[
            "typedef struct {\n    codec_variant_dispatch_body_kind_t kind;\n    \
             uint8_t default_tag;  /* the catch-all's */\n    union {\n        \
             codec_variant_session_open_source_ref_t codec_variant_session_open;\n        \
             codec_variant_session_close_source_ref_t codec_variant_session_close;\n        \
             codec_variant_session_close_source_ref_t default_body;\n    } arm;\n\
             } codec_variant_dispatch_body_ref_t;\n",
            "typedef struct {\n    codec_variant_dispatch_body_ref_t (*body)(const void *ctx);\n\
             } codec_variant_dispatch_source_t;\n",
            "    ref.kind = self->body.kind;\n    ref.default_tag = self->body.default_tag;\n    \
             switch (self->body.kind) {\n        \
             case CODEC_VARIANT_DISPATCH_BODY_KIND_CODEC_VARIANT_SESSION_OPEN:\n            \
             ref.arm.codec_variant_session_open.table = \
             codec_variant_session_open_value_source();\n            \
             ref.arm.codec_variant_session_open.ctx = \
             &self->body.arm.codec_variant_session_open;\n            break;\n",
            "    const codec_variant_dispatch_body_ref_t _e_body = src->body(ctx);\n    \
             uint64_t _t_body = 0;\n    switch (_e_body.kind) {\n        \
             case CODEC_VARIANT_DISPATCH_BODY_KIND_CODEC_VARIANT_SESSION_OPEN:\n            \
             _t_body = 0x1ull;\n            break;\n        \
             case CODEC_VARIANT_DISPATCH_BODY_KIND_CODEC_VARIANT_SESSION_CLOSE:\n            \
             _t_body = 0x2ull;\n            break;\n        \
             case CODEC_VARIANT_DISPATCH_BODY_KIND_DEFAULT:\n            \
             _t_body = (uint64_t)_e_body.default_tag;\n            break;\n        \
             default:\n            break;\n    }\n    \
             const uint8_t _d_msg_id = (uint8_t)_t_body;\n",
            "            SCE_FORGE_TRY_WRITE(codec_variant_session_close_encode_source(\
             _e_body.arm.default_body.table, _e_body.arm.default_body.ctx, w));\n",
            "            SCE_FORGE_TRY_WRITE(codec_variant_session_open_check_value(\
             &self->body.arm.codec_variant_session_open));\n",
            "    switch (_d_msg_id) {\n",
            "out->body.default_tag = _d_msg_id;",
        ],
    );
    holds_none(
        "codec_variant_dispatch",
        &text,
        &["    uint8_t msg_id;\n", "out->msg_id", "self->msg_id"],
    );
}

/// A variant whose arm's own first byte holds its tag derives no tag: the
/// encode writes the arm the source presents and nothing of its own.
///
/// Turns red where `encode_source` derives a tag for a variant with none of
/// its own, or stops writing the arm by its codec's encode.
#[test]
fn a_peeked_variant_writes_its_arm_alone() {
    assert!(unlowered("codec_variant_peek_basic").is_empty());
    let text = resource_header("codec_variant_peek_basic");
    holds(
        "codec_variant_peek_basic",
        &text,
        &["    switch (_e_body.kind) {\n        \
           case CODEC_VARIANT_PEEK_BASIC_BODY_KIND_CODEC_PEEK_ARM_A:\n            \
           SCE_FORGE_TRY_WRITE(codec_peek_arm_a_encode_source(_e_body.arm.codec_peek_arm_a.table, \
           _e_body.arm.codec_peek_arm_a.ctx, w));\n            break;\n"],
    );
    holds_none("codec_variant_peek_basic", &text, &["_t_body", "_d_peek"]);
}

/// A parent whose flag holds the tag of the variant of a body it embeds
/// derives the flag from the arm the body holds, read off the source the
/// body is presented as through the function of its codec's table that
/// presents the arm, and folds it into its carrier in place of the bits the
/// value presents there.
///
/// Turns red where `c11_source::parent_tag` reads the arm off anything but
/// the body's source, or names a kind other than the body's codec's, or where
/// `positional_forms` names a parent's tag as unlowered.
#[test]
fn a_parent_derives_the_flag_it_dispatches_on_from_the_arm_its_body_holds() {
    assert!(unlowered("codec_source_leaf").is_empty());
    assert!(unlowered("codec_source_parent_tag").is_empty());
    let text = resource_header("codec_source_parent_tag");
    holds(
        "codec_source_parent_tag",
        &text,
        &[
            "    uint64_t _p_hdr_kind = 0;\n    {\n        \
             const codec_source_leaf_body_ref_t _v = _e_body.table->body(_e_body.ctx);\n        \
             switch (_v.kind) {\n            \
             case CODEC_SOURCE_LEAF_BODY_KIND_CODEC_VARIANT_SESSION_OPEN:\n                \
             _p_hdr_kind |= 0x0ull;\n                break;\n            \
             case CODEC_SOURCE_LEAF_BODY_KIND_CODEC_VARIANT_SESSION_CLOSE:\n                \
             _p_hdr_kind |= 0x1ull;\n                break;\n            \
             default:\n                break;\n        }\n    }\n",
            "    _f_hdr = (_f_hdr & ~0x1ull) | _p_hdr_kind;\n",
        ],
    );
}

/// An input every arm of a variant says is what the arm the source presents
/// says, through its codec's own function of it; one some arm does not read
/// is the optional value, absent for that arm (docs/adr/0065 §6 R4d, R4e).
///
/// Turns red where `c11_source::arm_inputs` stops switching on the arm's kind,
/// reads an arm that does not bind the input, or answers a number where an
/// arm says nothing, or where `positional_forms` names such an input again.
#[test]
fn an_input_the_arms_say_is_read_through_the_arm_the_source_presents() {
    assert!(unlowered("codec_every_arm_says").is_empty());
    let every = resource_header("codec_every_arm_says");
    holds(
        "codec_every_arm_says",
        &every,
        &[
            "static inline uint8_t codec_every_arm_says_arm_input_n(\
             codec_every_arm_says_body_ref_t arm) {\n    switch (arm.kind) {\n",
            "            return codec_every_arm_says_far_input_bare(\
             arm.arm.codec_every_arm_says_far.table, arm.arm.codec_every_arm_says_far.ctx);\n",
            "static inline uint8_t codec_every_arm_says_input_n(\
             const codec_every_arm_says_source_t *src, const void *ctx) {\n    \
             return codec_every_arm_says_arm_input_n(src->body(ctx));\n}\n",
        ],
    );
    assert!(unlowered("codec_some_arms_say").is_empty());
    let some = resource_header("codec_some_arms_say");
    holds(
        "codec_some_arms_say",
        &some,
        &[
            "static inline sce_optional_uint8_t codec_some_arms_say_arm_input_n(\
             codec_some_arms_say_body_ref_t arm) {\n",
            "            return sce_optional_uint8_some(codec_every_arm_says_near_input_n(\
             arm.arm.codec_every_arm_says_near.table, arm.arm.codec_every_arm_says_near.ctx));\n",
            "    return sce_optional_uint8_none();\n}\n",
        ],
    );
    holds_none(
        "codec_some_arms_say",
        &some,
        &["codec_some_arms_say_plain_input_n("],
    );
}

/// A group of fields one flag decides is held as one member of the value, the
/// struct of its arm, absent where the flag allows no arm: the source presents
/// it as the value holds it, the encode binds each member off the arm to the
/// optional value a gated element is read into and derives the flag from it,
/// and the decode reads each member into a local and makes the arm the flag
/// it read selects. No member of the group is a member of the value's own.
///
/// Turns red where `c11_source::positional_forms` names a group of numbers
/// again, where `group::Group::declare` stops holding the members in the
/// arm's struct, where `member_read` binds a member off anything but the arm,
/// or where `Group::decoded` makes the arm of other than the decode's locals.
#[test]
fn a_group_is_held_as_the_struct_of_its_arm() {
    assert!(unlowered("codec_group_enum").is_empty());
    let text = resource_header("codec_group_enum");
    holds(
        "codec_group_enum",
        &text,
        &[
            "    uint8_t attempts;\n} codec_group_enum_flags_has_session_t;\n",
            "SCE_OPTIONAL_DECLARE_NAMED(codec_group_enum_flags_has_session_optional, \
             codec_group_enum_flags_has_session_t)\n",
            "    codec_group_enum_flags_has_session_optional_t flags_has_session;\n",
            "    codec_group_enum_flags_has_session_optional_t (*flags_has_session)(const void *ctx);\n",
            "    const codec_group_enum_flags_has_session_optional_t _e_flags_has_session = \
             src->flags_has_session(ctx);\n",
            "    const sce_optional_uint8_t _e_attempts = (_e_flags_has_session.present ? \
             sce_optional_uint8_some(_e_flags_has_session.value.attempts) : \
             sce_optional_uint8_none());\n",
            "uint8_t _d_attempts = 0;\n    if (",
            "    memset(&out->flags_has_session, 0, sizeof out->flags_has_session);\n    \
             if (((((uint64_t)out->flags >> 0) & 1u) != 0)) {\n        \
             out->flags_has_session.present = true;\n",
            "        out->flags_has_session.value.attempts = _d_attempts;\n",
        ],
    );
    holds_none(
        "codec_group_enum",
        &text,
        &["out->attempts", "self->attempts", "src->attempts"],
    );
}

/// A group of fields a flag input decides says the input: set exactly where
/// the value holds the group's arm, which the input's own function answers
/// for a parent, and the decode tests the input it is handed to make the arm.
///
/// Turns red where `c11_source::said_input` stops reading an input a group
/// decides off the group, or where `Group::decoded` tests an input other than
/// as the decode is handed it.
#[test]
fn an_input_a_group_decides_is_said_by_the_arm_the_value_holds() {
    assert!(unlowered("codec_init_syn_body").is_empty());
    let text = resource_header("codec_init_syn_body");
    holds(
        "codec_init_syn_body",
        &text,
        &[
            "static inline uint8_t codec_init_syn_body_input_s(\
             const codec_init_syn_body_source_t *src, const void *ctx) {\n    \
             return (uint8_t)((src->s(ctx).present) ? 1u : 0u);\n}\n",
            "    if ((s != 0)) {\n        out->s.present = true;\n",
        ],
    );
    holds_none(
        "codec_init_syn_body",
        &text,
        &["sce_forge_writer_t *w, uint8_t s)"],
    );
}

/// A group may hold a byte string on the side where its flag is set and the
/// length of it, which the value states, on the side where it is clear: the
/// arm holds the bytes in the buffer of their bound, a source presents them as
/// their view, the encode derives the length from the view where they are
/// there and takes the arm's where they are not, and the decode reads the
/// bytes in place into the arm it opened before them, and sets the length the
/// clear side states in its arm once every field is read.
///
/// Turns red where `c11_source::number_form` names a length a group holds
/// again, where `derive_number` stops reading `Absent::Group` off the member's
/// local, where `group::Group::opened` stops pointing the bytes at their arm,
/// where `present_if_decode_length_ref` reads a held member other than
/// through that pointer, or where `Group::decoded` sets the length in an arm
/// other than the one the decode opened.
#[test]
fn a_group_holds_bytes_in_place_and_their_stated_length_on_the_other_side() {
    let name = "codec_present_if_length_ref";
    assert!(unlowered(name).is_empty(), "{:?}", unlowered(name));
    let text = resource_header(name);
    holds(
        name,
        &text,
        &[
            "    sce_static_bytes_32_t payload;\n} \
             codec_present_if_length_ref_flags_has_payload_has_payload_t;\n",
            "    uint8_t payload_size;\n} \
             codec_present_if_length_ref_flags_has_payload_not_has_payload_t;\n",
            "    codec_present_if_length_ref_payload_bytes_t payload;\n} \
             codec_present_if_length_ref_flags_has_payload_has_payload_ref_t;\n",
            "    codec_present_if_length_ref_flags_has_payload_ref_t (*flags_has_payload)(const void *ctx);\n",
            "        ref.arm.has_payload.payload.data = \
             self->flags_has_payload.arm.has_payload.payload.data;\n",
            "    const codec_present_if_length_ref_payload_bytes_optional_t _e_payload = \
             (_e_flags_has_payload.kind == CODEC_PRESENT_IF_LENGTH_REF_FLAGS_HAS_PAYLOAD_HAS_PAYLOAD \
             ? codec_present_if_length_ref_payload_bytes_optional_some(\
             _e_flags_has_payload.arm.has_payload.payload) : \
             codec_present_if_length_ref_payload_bytes_optional_none());\n",
            "    const uint8_t _d_payload_size = _e_payload.present ? (uint8_t)(_e_payload.value.len) \
             : (_e_payload_size.present ? _e_payload_size.value : (uint8_t)0);\n",
            "    sce_static_bytes_32_t *_g_payload = NULL;\n",
            "        _g_payload = &out->flags_has_payload.arm.has_payload.payload;\n",
            "memcpy(_g_payload->data, raw, _n);\n        _g_payload->len = _n;",
            "    if (out->flags_has_payload.kind == \
             CODEC_PRESENT_IF_LENGTH_REF_FLAGS_HAS_PAYLOAD_NOT_HAS_PAYLOAD) {\n        \
             out->flags_has_payload.arm.not_has_payload.payload_size = _d_payload_size;\n    }",
        ],
    );
    holds_none(
        name,
        &text,
        &[
            "out->payload.",
            "self->payload",
            "out->payload_size",
            "self->payload_size",
        ],
    );
}

/// A group may hold a body on the side where its flag is set and the encoded
/// length of it, which the value states, on the side where it is clear: the
/// arm holds the body as its codec's struct, a source presents it as the pair
/// of its source, pointed at the body the arm holds, the encode counts the
/// body over a counting sink where it is there and writes it by its codec's
/// encode over that pair, and the decode reads it in place, within the length
/// it read, into the arm it opened.
///
/// Turns red where `group::Group::reader` stops pointing a body's pair at the
/// body the arm holds, where `encode_source` writes a body a group holds other
/// than through the pair the member is bound to, or where
/// `embed_streaming_decode_stmt` decodes a held body other than through the
/// pointer the opening set.
#[test]
fn a_group_holds_a_body_in_place_beside_its_stated_encoded_length() {
    let name = "codec_present_if_length_from";
    assert!(unlowered(name).is_empty(), "{:?}", unlowered(name));
    let text = resource_header(name);
    holds(
        name,
        &text,
        &[
            "    codec_zenoh_timestamp_t ts;\n} \
             codec_present_if_length_from_flags_has_ts_has_ts_t;\n",
            "        ref.arm.has_ts.ts.table = codec_zenoh_timestamp_value_source();\n        \
             ref.arm.has_ts.ts.ctx = &self->flags_has_ts.arm.has_ts.ts;\n",
            "    const uint64_t _d_ext_size = _e_ts.present ? \
             (uint64_t)(sce_forge_writer_position(&_k_ts)) : \
             (_e_ext_size.present ? _e_ext_size.value : (uint64_t)0);\n",
            "SCE_FORGE_TRY_WRITE(codec_zenoh_timestamp_encode_source(_e_ts.value.table, \
             _e_ts.value.ctx, w));\n",
            "    codec_zenoh_timestamp_t *_g_ts = NULL;\n",
            "        _g_ts = &out->flags_has_ts.arm.has_ts.ts;\n",
            "        size_t _len = (size_t)(_d_ext_size);\n",
            "sce_forge_codec_status_t _st = codec_zenoh_timestamp_decode(&_inner, _g_ts);",
            "out->flags_has_ts.arm.not_has_ts.ext_size = _d_ext_size;\n",
        ],
    );
    holds_none(name, &text, &["out->ts", "self->ts", "out->ext_size"]);
}

/// A list whose entries say the input it hands them is held to its law: the
/// list's type holds the start every entry says, its maker takes the start and
/// refuses an entry that says another, the value holds the start beside the
/// list, which the decode sets from the flag it handed the entries and the
/// value's check holds the entries to by the maker, and the encode derives the
/// flag bound to the input from the start.
///
/// Turns red where `c11_source::positional_forms` names a list held to a law
/// again, where `law_text` stops holding an entry to the start, where
/// `start_members` or `decoded` stops holding the start in the value, or where
/// `measure_text` reads `Measured::Start` other than off the list.
#[test]
fn a_list_held_to_a_law_is_made_from_its_start() {
    let name = "codec_repeat_flag_carrier";
    assert!(unlowered(name).is_empty(), "{:?}", unlowered(name));
    let text = resource_header(name);
    holds(
        name,
        &text,
        &[
            "    size_t count;\n    uint8_t start_wide;\n} codec_repeat_flag_carrier_entries_list_t;\n",
            "static inline sce_forge_codec_status_t codec_repeat_flag_carrier_entries_list_make(\
             codec_repeat_flag_carrier_entries_list_t *out, const sce_forge_walk_t *walk, \
             const void *ctx, const codec_chain_flag_entry_source_t *entry, uint8_t start_wide) {\n",
            "        if (((codec_chain_flag_entry_input_wide(entry, at)) != 0) != (start_wide != 0)) {\n            \
             return SCE_FORGE_CODEC_PRESENT_IF_MISMATCH;\n        }\n",
            "    out->start_wide = start_wide;\n",
            "    uint8_t entries_start_wide;\n",
            "    list.start_wide = self->entries_start_wide;\n",
            "    _f_head = (_f_head & ~0x1ull) | (uint64_t)((size_t)_e_entries.start_wide);\n",
            "out->entries_start_wide = (uint8_t)(((((uint64_t)out->head >> 0) & 1u) != 0) ? 1u : 0u);",
            "SCE_FORGE_TRY_WRITE(codec_repeat_flag_carrier_entries_list_make(&_made, _l.walk, \
             _l.ctx, _l.entry, _l.start_wide));\n",
        ],
    );
}

/// A chain that hands its entries an input nothing binds hands each whether
/// the entry before it was of the kind the input waits for, the first entry's
/// clear, read off the kind the entry before presents.
///
/// Turns red where `c11_source::positional_forms` names a chain handed an
/// input nothing binds again, where `handed_args` stops passing the walk's
/// local, or where the walk stops setting it from the entry it wrote.
#[test]
fn a_chain_hands_each_entry_what_the_entry_before_was() {
    let name = "codec_chain_handed";
    assert!(unlowered(name).is_empty(), "{:?}", unlowered(name));
    let text = resource_header(name);
    holds(
        name,
        &text,
        &[
            "        uint8_t _p_after_marker = 0;\n",
            "SCE_FORGE_TRY_WRITE(codec_chain_handed_entry_encode_source_overriding(\
             _e_entries.entry, _at, w, _p_after_marker, _o));\n",
            "            _p_after_marker = (uint8_t)(((((uint64_t)_e_entries.entry->header(_at)) \
             >> 0) & 0xFull) == 0x4ull);\n",
        ],
    );
}

/// A codec whose plan uses a form the C11 printer does not lower keeps the
/// template's encode whole: a group of a chain's memberships and a CBOR map
/// each name their form, and neither has an encode over a source.
///
/// Turns red where `c11_source::positional_forms` stops naming a group of a
/// chain's memberships, or `unlowered_forms` a CBOR map, or where the
/// template prints the encode over a source for a codec it keeps.
#[test]
fn a_codec_of_a_form_the_printer_does_not_lower_keeps_the_template_encode() {
    for (name, form) in [
        ("codec_chain_has_marker", "a group of a chain's memberships"),
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
        "codec_embed_basic",
        "codec_zenoh_timestamp_ext",
        "codec_length_shared",
        "codec_length_stated",
        "codec_encoded_length_checked",
        "codec_repeat_basic",
        "codec_repeat_present_if_basic",
        "codec_until_eof_basic",
        "codec_tlv_chain_basic",
        "codec_tlv_chain_present_if_basic",
        "codec_nested_body",
        "codec_nested_parent",
        "codec_origin_envelope",
        "codec_chain_depth_entry",
        "codec_chain_depth_plain",
        "codec_chain_depth_gated",
        "codec_zenoh_wireexpr",
        "codec_zenoh_decl_kexpr",
        "codec_init_cookie_body",
        "codec_variant_dispatch",
        "codec_origin_choice",
        "codec_variant_peek_basic",
        "codec_variant_default_marker",
        "codec_source_leaf",
        "codec_source_parent_tag",
        "codec_every_arm_says",
        "codec_every_arm_says_parent",
        "codec_some_arms_say",
        "codec_some_arms_say_parent",
        "codec_some_arms_say_forward",
        "codec_group_enum",
        "codec_init_syn_body",
        "codec_init_syn_envelope",
        "codec_present_if_length_ref",
        "codec_present_if_length_from",
        "codec_repeat_flag_carrier",
        "codec_chain_handed",
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
#include "codec_repeat_basic.h"
#include "codec_simple_frame.h"
#include "codec_variant_dispatch.h"

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

/* A chain of nodes of its own, each holding a sequence number: a list the codec
 * walks where it is held, by a begin and next pair of its own. */
struct node {
    unsigned seq;
    const struct node *next;
};

static uint16_t node_seq(const void *ctx) {
    return (uint16_t)((const struct node *)ctx)->seq;
}

static const codec_repeat_elem_source_t node_source = {
    node_seq,
};

static const void *nodes_begin(const void *ctx) {
    return ctx;
}

static const void *nodes_next(const void *ctx, const void *entry) {
    (void)ctx;
    return ((const struct node *)entry)->next;
}

static const sce_forge_walk_t nodes_walk = {
    nodes_begin,
    nodes_next,
};

/* A batch of its own, its list made once by the codec's maker. */
struct batch {
    codec_repeat_basic_frags_list_t frags;
};

static codec_repeat_basic_frags_list_t batch_frags(const void *ctx) {
    return ((const struct batch *)ctx)->frags;
}

static const codec_repeat_basic_source_t batch_source = {
    batch_frags,
};

/* An opening of its own, presented as the open arm of the dispatch: the arm's
 * kind beside a source of the arm's codec, by a table of its own. */
struct opening {
    unsigned version;
};

static uint16_t opening_version(const void *ctx) {
    return (uint16_t)((const struct opening *)ctx)->version;
}

static const codec_variant_session_open_source_t opening_source = {
    opening_version,
};

static codec_variant_dispatch_body_ref_t opening_body(const void *ctx) {
    codec_variant_dispatch_body_ref_t ref;
    memset(&ref, 0, sizeof ref);
    ref.kind = CODEC_VARIANT_DISPATCH_BODY_KIND_CODEC_VARIANT_SESSION_OPEN;
    ref.arm.codec_variant_session_open.table = &opening_source;
    ref.arm.codec_variant_session_open.ctx = ctx;
    return ref;
}

static const codec_variant_dispatch_source_t dispatch_source = {
    opening_body,
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

    /* A list is walked where it is held: the count is what the list's maker
     * counted, and each entry is written through the table of its own. */
    const struct node second = {0xFACEu, NULL};
    const struct node first = {0xBEEFu, &second};
    struct batch b;
    CHECK(codec_repeat_basic_frags_list_make(&b.frags, &nodes_walk, &first, &node_source) == SCE_FORGE_CODEC_OK,
          "the nodes make a list");
    static const uint8_t two[] = {0x02, 0xBE, 0xEF, 0xFA, 0xCE};
    uint8_t by_batch[CODEC_REPEAT_BASIC_MAX_BYTES];
    sce_forge_writer_t bw = sce_forge_writer_init_buf(by_batch, sizeof by_batch);
    CHECK(codec_repeat_basic_encode_source(&batch_source, &b, &bw) == SCE_FORGE_CODEC_OK, "the batch encodes");
    CHECK(bw.pos == sizeof two && memcmp(by_batch, two, sizeof two) == 0,
          "the batch writes the count its maker found and each node");

    /* A walk that never ends is refused once it passes the bound. */
    struct node loop = {0x0001u, NULL};
    loop.next = &loop;
    CHECK(codec_repeat_basic_frags_list_make(&b.frags, &nodes_walk, &loop, &node_source) ==
              SCE_FORGE_CODEC_TOO_MANY_ELEMENTS,
          "a walk past the bound is refused");

    /* The arm a record of its own presents decides the tag the frame holds,
     * and the arm's body is written through the arm's table. */
    const struct opening o = {0x0102u};
    static const uint8_t opened[] = {0x01, 0x01, 0x02};
    uint8_t by_opening[CODEC_VARIANT_DISPATCH_MAX_BYTES];
    sce_forge_writer_t ow = sce_forge_writer_init_buf(by_opening, sizeof by_opening);
    CHECK(codec_variant_dispatch_encode_source(&dispatch_source, &o, &ow) == SCE_FORGE_CODEC_OK,
          "the opening encodes");
    CHECK(ow.pos == sizeof opened && memcmp(by_opening, opened, sizeof opened) == 0,
          "the opening writes the tag of its arm, then the arm");

    /* A value of the struct holding the catch-all writes the tag it holds. */
    static codec_variant_dispatch_t dv;
    memset(&dv, 0, sizeof dv);
    dv.body.kind = CODEC_VARIANT_DISPATCH_BODY_KIND_DEFAULT;
    dv.body.default_tag = 0x7Fu;
    dv.body.arm.default_body.reason = 0x09u;
    static const uint8_t unknown[] = {0x7F, 0x09};
    uint8_t by_default[CODEC_VARIANT_DISPATCH_MAX_BYTES];
    size_t dn = 0;
    CHECK(codec_variant_dispatch_encode_to_buf(&dv, by_default, sizeof by_default, &dn) == SCE_FORGE_CODEC_OK &&
              dn == sizeof unknown && memcmp(by_default, unknown, dn) == 0,
          "the catch-all writes the tag it holds");
    static codec_variant_dispatch_t dback;
    memset(&dback, 0, sizeof dback);
    sce_forge_cursor_t dc = sce_forge_cursor_init(unknown, sizeof unknown);
    CHECK(codec_variant_dispatch_decode(&dc, &dback) == SCE_FORGE_CODEC_OK &&
              dback.body.kind == CODEC_VARIANT_DISPATCH_BODY_KIND_DEFAULT && dback.body.default_tag == 0x7Fu &&
              dback.body.arm.default_body.reason == 0x09u,
          "the decode dispatches on the tag it read into a local");

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
    for name in [
        "codec_simple_frame",
        "codec_length_ref",
        "codec_repeat_elem",
        "codec_repeat_basic",
        "codec_variant_session_open",
        "codec_variant_session_close",
        "codec_variant_dispatch",
    ] {
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
