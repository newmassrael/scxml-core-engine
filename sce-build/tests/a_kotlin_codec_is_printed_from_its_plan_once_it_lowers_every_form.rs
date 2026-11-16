// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Which codecs the Kotlin backend prints from their encode plan
//! (docs/adr/0081 step 4, §6), and what a printed one is.
//!
//! The Kotlin printer lowers the plan's forms one landing at a time. A codec
//! whose every form it lowers is printed: its file names an interface of the
//! elements a source presents, which the codec's data class implements, an
//! encode over any source that derives every other element of the frame
//! (docs/adr/0058 §1), and a data class that holds no element the encode
//! derives and holds each byte string in the value class of its bound, made
//! only by a checked factory. Every other codec keeps the template's encode
//! whole, and the printer names the first form it does not lower.

use std::path::{Path, PathBuf};

use sce_build::generator::Language;
use sce_build::{
    compile_forge_file, kotlin_codec_printed_file, kotlin_codec_unlowered_file, ForgeCompileOptions,
};

/// The directory of the resources.
fn resources() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/resources")
}

/// The Kotlin file the document `path` generates, the one that declares its
/// data class.
fn file_of(path: &Path) -> Result<String, String> {
    let stem = path
        .file_stem()
        .expect("a document has a stem")
        .to_string_lossy()
        .into_owned();
    let output = compile_forge_file(path, Language::Kotlin, &[], &ForgeCompileOptions::default())
        .map_err(|e| format!("{:?}", e.error))?;
    output
        .files
        .into_iter()
        .find(|(file, _)| file.ends_with(".kt"))
        .map(|(_, text)| text)
        .ok_or_else(|| format!("{stem} generates no Kotlin file"))
}

/// The Kotlin file of the resource `name`, which generates.
fn resource_file(name: &str) -> String {
    file_of(&resources().join(format!("{name}.scxml")))
        .unwrap_or_else(|e| panic!("{name} generates on Kotlin: {e}"))
}

/// The forms of the resource `name` the Kotlin printer does not lower.
fn unlowered(name: &str) -> Vec<String> {
    kotlin_codec_unlowered_file(
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
            "{name}: the file holds `{needle}`:\n{text}"
        );
    }
}

/// Whether `text` holds none of `needles`, with `text` in the failure.
fn holds_none(name: &str, text: &str, needles: &[&str]) {
    for needle in needles {
        assert!(
            !text.contains(needle),
            "{name}: the file holds no `{needle}`:\n{text}"
        );
    }
}

/// A length of a byte string is derived on encode from the bytes the source
/// presents, and is no field of the value; the byte string is presented in the
/// value class of its bound, which its checked factory alone makes.
///
/// Turns red where `kotlin_source::encode_source` reads the length off the
/// source rather than measuring the bytes, where the template's data class
/// loop stops skipping a derived field, or where `render` declares no value
/// class of the bound.
#[test]
fn a_length_is_derived_from_the_bytes_it_measures_and_is_no_field_of_the_value() {
    assert!(unlowered("codec_length_ref").is_empty());
    let text = resource_file("codec_length_ref");
    holds(
        "codec_length_ref",
        &text,
        &[
            "import kotlin.jvm.JvmInline\n",
            "@JvmInline\nvalue class CodecLengthRefPayloadBytes private constructor(val bytes: ByteArray) {\n",
            "        const val MOST: Int = 32\n",
            "        fun of(bytes: ByteArray): CodecLengthRefPayloadBytes? =\n            \
             if (bytes.size <= MOST) CodecLengthRefPayloadBytes(bytes) else null\n",
            "interface CodecLengthRefSource {\n    val msgId: UByte\n    \
             val payload: CodecLengthRefPayloadBytes\n}\n",
            "    override var payload: CodecLengthRefPayloadBytes = \
             CodecLengthRefPayloadBytes.fewest()\n",
            ") : CodecLengthRefSource {\n",
            "        return encodeSource(this, w)\n",
            "        fun encodeSource(src: CodecLengthRefSource, w: SceSink): CodecError? {\n",
            "            val _e_payload = src.payload\n",
            "            val _d_len = _e_payload.bytes.size.toUByte()\n",
            "            w.writeU8(_d_len.toByte())?.let { return it }\n",
            "            w.writeBytes(_e_payload.bytes)?.let { return it }\n",
            "                payload = CodecLengthRefPayloadBytes.of(payload) ?: return null\n",
            "    fun encodeToByteArray(): ByteArray {\n",
        ],
    );
    // The value holds no length, is made without one, and the encode reads
    // none off the value; it refuses no bound, which the class holds.
    holds_none(
        "codec_length_ref",
        &text,
        &[
            "var len: ",
            "                len = len",
            "src.len",
            "if (this.payload.size > 32)",
            "return CodecError.LengthOutOfRange",
        ],
    );
}

/// A length its `sce:length-arith` takes off is derived less that, and the
/// value class of the bytes holds at least what it takes off, so a value made
/// by default is within its bound.
///
/// Turns red where `kotlin_source::encode_source` drops the `Offset::Less`
/// arithmetic, or where `bounded_bytes_class` stops testing a least past zero.
#[test]
fn a_length_less_its_arithmetic_is_derived_less_it() {
    assert!(unlowered("codec_scout_zid_body").is_empty());
    let text = resource_file("codec_scout_zid_body");
    holds(
        "codec_scout_zid_body",
        &text,
        &[
            "        const val LEAST: Int = 1\n",
            "if (bytes.size in LEAST..MOST) CodecScoutZidBodyZidBytes(bytes) else null\n",
            "        fun fewest(): CodecScoutZidBodyZidBytes = \
             CodecScoutZidBodyZidBytes(ByteArray(LEAST))\n",
            "(_e_zid.bytes.size - 1)",
        ],
    );
    holds_none(
        "codec_scout_zid_body",
        &text,
        &["var zid_len_m1: ", "src.zid_len_m1"],
    );
}

/// A codec of integers alone is printed with no value class, its data class's
/// fields its elements, each overriding the source's property.
///
/// Turns red where `kotlin_source::positional_forms` refuses an integer
/// element, or where the template imports `JvmInline` for a codec that
/// presents no byte string.
#[test]
fn a_codec_of_integers_is_printed_with_no_value_class() {
    assert!(unlowered("codec_simple_frame").is_empty());
    let text = resource_file("codec_simple_frame");
    holds(
        "codec_simple_frame",
        &text,
        &[
            "interface CodecSimpleFrameSource {\n",
            ") : CodecSimpleFrameSource {\n",
            "    override var msgId: ",
            "        fun encodeSource(src: CodecSimpleFrameSource, w: SceSink): CodecError? {\n",
            "        return encodeSource(this, w)\n",
        ],
    );
    holds_none(
        "codec_simple_frame",
        &text,
        &["JvmInline", "value class", "_d_"],
    );
}

/// A text is presented in the value class of its bound, counted in the bytes
/// UTF-8 encodes it to, which the runtime counts and refuses a text no UTF-8
/// holds; its length is derived from those bytes, and the bytes are written.
///
/// Turns red where `kotlin_source::bounded_class` counts a text in UTF-16
/// units rather than through `Utf8.size`, or `encode_source` measures or
/// writes a text other than as its UTF-8 bytes.
#[test]
fn a_text_is_held_to_its_bound_in_utf8_bytes_and_its_length_derived_from_them() {
    assert!(unlowered("codec_zenoh_locator").is_empty());
    let text = resource_file("codec_zenoh_locator");
    holds(
        "codec_zenoh_locator",
        &text,
        &[
            "value class CodecZenohLocatorLocatorText private constructor(val text: String) {\n",
            "            val size = com.sce.forge.runtime.Utf8.size(text) ?: return null\n",
            "            return if (size <= MOST) CodecZenohLocatorLocatorText(text) else null\n",
            "    val locator: CodecZenohLocatorLocatorText\n",
            "val _d_locator_len = _e_locator.text.encodeToByteArray().size.toULong()\n",
            "w.writeVleU64(_d_locator_len)?.let { return it }\n",
            "w.writeBytes(_e_locator.text.encodeToByteArray())?.let { return it }\n",
            "                locator = CodecZenohLocatorLocatorText.of(locator) ?: return null",
        ],
    );
    holds_none(
        "codec_zenoh_locator",
        &text,
        &["var locator_len: ", "src.locator_len"],
    );
}

/// An element a gate decides is `null` where it is absent, written only where
/// it is there, and the flag that gates it is derived from it and folded into
/// its carrier in place of the value's bit.
///
/// Turns red where `kotlin_source::encode_source` reads the presence flag off
/// the value rather than folding it from the gated element, or writes the
/// gated element whether or not it is there.
#[test]
fn a_gated_element_derives_its_flag_and_is_written_where_it_is_there() {
    assert!(unlowered("codec_present_if_basic").is_empty());
    let text = resource_file("codec_present_if_basic");
    holds(
        "codec_present_if_basic",
        &text,
        &[
            "    val seq: UShort?\n",
            "            val _e_seq = src.seq\n",
            "            var _u_flags = _e_flags.toUInt()\n",
            "            _u_flags = (_u_flags and 0x1u.inv()) or \
             (if (_e_seq != null) 0x1u else 0x0u)\n",
            "            val _c_flags = _u_flags.toUByte()\n",
            "            w.writeU8(_c_flags.toByte())?.let { return it }\n",
            "            if (_e_seq != null) {\n                w.writeU8(",
        ],
    );
    holds_none("codec_present_if_basic", &text, &["src.flags.toByte()"]);
}

/// A body of a printed codec is any source of that codec, the interface its
/// file declares, and is written by that codec's companion `encodeSource`; the
/// value holds the body's data class, which implements the interface.
///
/// Turns red where `kotlin_source::positional_forms` stops printing a body of a
/// printed codec, or `encode_source` writes it other than through its codec's
/// `encodeSource`.
#[test]
fn a_body_is_written_by_its_codecs_own_encode_over_a_source() {
    assert!(unlowered("codec_embed_basic").is_empty());
    let text = resource_file("codec_embed_basic");
    holds(
        "codec_embed_basic",
        &text,
        &[
            "import com.sce.generated.codec_zenoh_locator.*\n",
            "    val locator: CodecZenohLocatorSource\n",
            "    override var locator: CodecZenohLocator = CodecZenohLocator()",
            "CodecZenohLocator.encodeSource(_e_locator, w)?.let { return it }\n",
        ],
    );
}

/// The encoded length of a body is the length of its frame, encoded once into a
/// buffer that is then written where the body is.
///
/// Turns red where `kotlin_source::encode_source` reads the length off the
/// value, or writes the body by encoding it a second time.
#[test]
fn a_bodys_encoded_length_is_measured_over_the_frame_it_writes() {
    assert!(unlowered("codec_zenoh_timestamp_ext").is_empty());
    let text = resource_file("codec_zenoh_timestamp_ext");
    holds(
        "codec_zenoh_timestamp_ext",
        &text,
        &[
            "val _l_ts = mutableListOf<Byte>()\n",
            "CodecZenohTimestamp.encodeSource(_e_ts, MutableListSink(_l_ts))?.let { return it }\n",
            "val _b_ts = _l_ts.toByteArray()\n",
            "val _d_ext_size = _b_ts.size.toULong()\n",
            "w.writeVleU64(_d_ext_size)?.let { return it }\n",
            "w.writeBytes(_b_ts)?.let { return it }\n",
        ],
    );
    holds_none(
        "codec_zenoh_timestamp_ext",
        &text,
        &["var ext_size: ", "src.ext_size", "this.ts.encode(w)"],
    );
}

/// A list of a printed codec is held to its bound in a value class over a list
/// of sources of the entries' codec, copied where it is made; its count is
/// derived from how many entries it holds, and each entry is written by its
/// codec's own encode.
///
/// Turns red where `kotlin_source::encode_source` reads the count off the value
/// or stops walking the list through its entries' codec, or where
/// `bounded_list_class` keeps the list it was handed rather than a copy.
#[test]
fn a_list_derives_its_count_and_writes_each_entry_by_its_codec() {
    assert!(unlowered("codec_repeat_basic").is_empty());
    let text = resource_file("codec_repeat_basic");
    holds(
        "codec_repeat_basic",
        &text,
        &[
            "value class CodecRepeatBasicFragsList private constructor(\
             val entries: List<CodecRepeatElemSource>) {\n",
            "        const val MOST: Int = 32\n",
            "if (entries.size <= MOST) CodecRepeatBasicFragsList(entries.toList()) else null\n",
            "    val frags: CodecRepeatBasicFragsList\n",
            "    override var frags: CodecRepeatBasicFragsList = CodecRepeatBasicFragsList.fewest()",
            "val _d_num_frags = _e_frags.entries.size.toUByte()\n",
            "for (_entry in _e_frags.entries) {\n",
            "CodecRepeatElem.encodeSource(_entry, w)?.let { return it }\n",
            "frags = CodecRepeatBasicFragsList.of(frags) ?: return null",
        ],
    );
    holds_none(
        "codec_repeat_basic",
        &text,
        &["var num_frags: ", "src.num_frags"],
    );
}

/// A gate the encode cannot derive the presence from is a check: the value is
/// held to it before the first byte, tested on the carrier as the encode writes
/// it, and refused with the runtime's `CodecError.PresentIfMismatch`.
///
/// Turns red where `kotlin_source::encode_source` drops the loop over
/// `plan.gate_checks`, tests the gate on the value's carrier rather than the
/// one the encode writes, or `positional_forms` names a checked gate unlowered.
#[test]
fn a_gate_the_encode_cannot_derive_is_held_to_the_field_it_gates() {
    assert!(unlowered("codec_present_if_checked").is_empty());
    let text = resource_file("codec_present_if_checked");
    holds(
        "codec_present_if_checked",
        &text,
        &[
            "            _u_flags = (_u_flags and 0x1u.inv()) or \
             (if (_e_seq != null) 0x1u else 0x0u)\n",
            "            if ((((_c_flags.toUInt() shr 0) and 0x1u) != 0x0u || \
             ((_c_flags.toUInt() shr 1) and 0x1u) != 0x0u) != (_e_trailer != null)) {\n\
             \x20               return CodecError.PresentIfMismatch\n",
            "    fun encodeToByteArray(): ByteArray? {\n",
        ],
    );
}

/// Two byte strings that share one length each measure it, and a value whose
/// two differ is refused as an inconsistent source before the first byte.
///
/// Turns red where `kotlin_source::derive_number` stops comparing every
/// measure always there with the first, or takes the number from the second.
#[test]
fn a_length_two_byte_strings_share_refuses_two_that_differ() {
    assert!(unlowered("codec_length_shared").is_empty());
    let text = resource_file("codec_length_shared");
    holds(
        "codec_length_shared",
        &text,
        &[
            "            if (_e_beta.bytes.size != _e_alpha.bytes.size) {\n\
             \x20               return CodecError.InconsistentSource\n",
            "            val _d_span_len = _e_alpha.bytes.size",
        ],
    );
    holds_none("codec_length_shared", &text, &["src.span_len"]);
}

/// The encoded length of a body that can refuse is refused where it does not
/// fit its field, and a frame that holds a codec that can refuse is written
/// whole first and handed to the caller's sink only once it is, so a refusal
/// leaves that sink as it was.
///
/// Turns red where `kotlin_source::derive_number` drops the `Narrow::Checked`
/// test, or `encode_source` writes straight into the caller's sink for a plan
/// with a `RefusalKind::Child` refusal.
#[test]
fn a_frame_that_holds_a_codec_that_can_refuse_is_written_whole_first() {
    assert!(unlowered("codec_encoded_length_checked").is_empty());
    let text = resource_file("codec_encoded_length_checked");
    holds(
        "codec_encoded_length_checked",
        &text,
        &[
            "            val _l_inner = mutableListOf<Byte>()\n\
             \x20           CodecLengthShared.encodeSource(_e_inner, MutableListSink(_l_inner))\
             ?.let { return it }\n\
             \x20           val _b_inner = _l_inner.toByteArray()\n",
            "            if (_b_inner.size > 0xFFFF) {\n\
             \x20               return CodecError.LengthOutOfRange\n",
            "            val _f = mutableListOf<Byte>()\n            val _s = MutableListSink(_f)\n",
            "            _s.writeBytes(_b_inner)?.let { return it }\n",
            "            w.writeBytes(_f.toByteArray())?.let { return it }\n",
        ],
    );
}

/// A length the value states while the byte strings it measures are absent is
/// `null` while any of them is there, derived then from them, each one there
/// saying the same, and what the value states where none is; a value that
/// states it beside content that says otherwise is refused, and the decode
/// gives it back `null` where it read any of the content.
///
/// Turns red where `kotlin_source::derive_number` reads the stated number
/// where content is there, drops the check of a stated number against the
/// content, `render` decodes the number where the content was read, or
/// `derived_field` drops the stated number from the data class.
#[test]
fn a_length_the_value_states_is_read_only_where_its_content_is_absent() {
    assert!(unlowered("codec_length_stated").is_empty());
    let text = resource_file("codec_length_stated");
    holds(
        "codec_length_stated",
        &text,
        &[
            "    val span_len: UByte?\n",
            "    override var span_len: UByte? = null",
            "            var _f_span_len: Int? = null\n\
             \x20           if (_e_alpha != null) {\n\
             \x20               _f_span_len = _e_alpha.bytes.size\n\
             \x20           } else if (_e_beta != null) {\n\
             \x20               _f_span_len = _e_beta.bytes.size\n\
             \x20           }\n",
            "            if (_e_beta != null && _e_beta.bytes.size != _f_span_len) {\n\
             \x20               return CodecError.InconsistentSource\n",
            "            val _d_span_len = if (_f_span_len != null) _f_span_len.toUByte() \
             else (_e_span_len ?: 0.toUByte())\n",
            "            if (_f_span_len != null && _e_span_len != null && \
             _e_span_len != _d_span_len) {\n",
            "                span_len = if (alpha == null && beta == null) span_len else null",
        ],
    );
}

/// A length held in a flag of a carrier is folded into the carrier's bits from
/// the bytes it measures, in place of the bits the value presents, and the
/// value's other flags of that carrier are written as it presents them.
///
/// Turns red where `kotlin_source::derive_number` stops folding a
/// `NumberInto::Flag` number into its carrier, or `number_form` names it
/// unlowered.
#[test]
fn a_length_held_in_a_flag_is_folded_into_its_carrier() {
    assert!(unlowered("codec_length_ref_dotted_basic").is_empty());
    let text = resource_file("codec_length_ref_dotted_basic");
    holds(
        "codec_length_ref_dotted_basic",
        &text,
        &[
            "            var _u_carrier = _e_carrier.toUInt()\n",
            "            _u_carrier = (_u_carrier and 0xF0u.inv()) or \
             ((_e_payload.bytes.size).toUInt() shl 4)\n",
            "            val _c_carrier = _u_carrier.toUByte()\n",
            "            w.writeBytes(_e_payload.bytes)?.let { return it }\n",
        ],
    );
}

/// A flag input the value does not say is a parameter of the encode after the
/// sink, which a gate tests as it tests a flag, and which the value's `encode`
/// hands on; an input the value says is not printed yet.
///
/// Turns red where `kotlin_source::encode_source` drops `plan.given` from its
/// parameters or `gate_test` stops spelling a `Clause::Input`, where the
/// template's `encode` stops handing its inputs on, or where
/// `positional_forms` stops naming an input the value says.
#[test]
fn a_flag_input_the_value_does_not_say_is_handed_to_the_encode() {
    assert!(unlowered("codec_chain_handed_entry").is_empty());
    let text = resource_file("codec_chain_handed_entry");
    holds(
        "codec_chain_handed_entry",
        &text,
        &[
            "        fun encodeSource(src: CodecChainHandedEntrySource, w: SceSink, \
             afterMarker: UByte): CodecError? {\n",
            "            if ((afterMarker.toUInt() != 0u || ((_e_header.toUInt() shr 6) and 0x1u) \
             != 0x0u) != (_e_value != null)) {\n\
             \x20               return CodecError.PresentIfMismatch\n",
            "        return encodeSource(this, w, afterMarker)\n",
        ],
    );
    let forms = unlowered("codec_zenoh_init_body");
    assert!(
        forms
            .iter()
            .any(|f| f.contains("a flag input the value says")),
        "codec_zenoh_init_body says S and A by the fields gated on them: {forms:?}"
    );
}

/// A codec whose one field is gated on a flag input alone says the input
/// (docs/adr/0065 §6 R4a): its encode over a source is not handed it and reads
/// it off the field, its companion answers it off any source for a parent, and
/// the value's own `encode`, which its callers hand every input, holds what it
/// is handed to what the value says.
///
/// Turns red where `kotlin_source::positional_forms` names an input a gated
/// field says unlowered, `encode_source` takes it as a parameter, `said_value`
/// reads it other than off the field's presence, or `render` stops holding a
/// handed input to it.
#[test]
fn an_input_the_value_says_is_read_off_it_and_not_handed() {
    const SAYS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="said_record" version="1.0">
  <sce:flag-inputs>
    <sce:flag-input name="S" width="1"/>
  </sce:flag-inputs>
  <datamodel>
    <sce:field id="version" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:field id="detail" sce:type="uint8" sce:byte="1" sce:bit-size="8"
               sce:present-if="S"/>
  </datamodel>
</scxml>"#;
    let output = sce_build::compile_forge_from_string(
        SAYS,
        sce_build::DocumentLabel::symmetric("said_record"),
        Language::Kotlin,
    )
    .unwrap_or_else(|e| panic!("said_record generates on Kotlin: {e:?}"));
    let text = output
        .files
        .into_iter()
        .find(|(file, _)| file.ends_with(".kt"))
        .map(|(_, text)| text)
        .expect("said_record generates a Kotlin file");
    holds(
        "said_record",
        &text,
        &[
            "        fun encodeSource(src: SaidRecordSource, w: SceSink): CodecError? {\n",
            "        fun inputS(src: SaidRecordSource): UByte =\n            \
             (if (src.detail != null) 1 else 0).toUByte()",
            "        if (s != inputS(this)) return CodecError.PresentIfMismatch\n\
             \x20       return encodeSource(this, w)\n",
            "    fun encodeToByteArray(s: UByte): ByteArray? {\n",
        ],
    );
}

/// A codec whose plan uses a form the Kotlin printer does not lower keeps the
/// template's encode whole: a variant, a CBOR map and a group that holds a
/// length the value states beside its content each name their form.
///
/// Turns red where `kotlin_source::positional_forms` stops naming a variant or
/// a group, or `unlowered_forms` a CBOR map.
#[test]
fn a_codec_of_a_form_the_printer_does_not_lower_keeps_the_template_encode() {
    for (name, form) in [
        ("codec_variant_default_marker", "a variant"),
        ("codec_cbor_map", "a CBOR map"),
        ("codec_present_if_length_ref", "a group"),
    ] {
        let forms = unlowered(name);
        assert!(
            forms.iter().any(|f| f.contains(form)),
            "{name}: the printer names `{form}` among {forms:?}"
        );
        let text = resource_file(name);
        holds_none(
            name,
            &text,
            &["fun encodeSource(", "Source {\n", "@JvmInline"],
        );
    }
}

/// Every codec of the resources is printed exactly where the printer lowers
/// each of its forms, and each printed one generates a file with its encode
/// over a source. The printed codecs are shown with `--nocapture`.
///
/// Turns red where `kotlin_codec_printed` and `kotlin_codec_unlowered` come
/// apart, where `render_codec` prints a codec the printer does not lower or
/// leaves one it lowers on the template's encode, or where the printer panics
/// or the template fails on a codec it lowers.
#[test]
fn the_kotlin_backend_prints_each_codec_of_the_resources_whose_forms_it_lowers() {
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
        let Ok(Some(forms)) =
            kotlin_codec_unlowered_file(path, &[], &ForgeCompileOptions::default())
        else {
            continue;
        };
        assert_eq!(
            kotlin_codec_printed_file(path, &[], &ForgeCompileOptions::default()).ok(),
            Some(Some(forms.is_empty())),
            "{stem}: printed exactly where no form is unlowered"
        );
        match file_of(path) {
            Ok(text) => assert_eq!(
                text.contains("        fun encodeSource(src: "),
                forms.is_empty(),
                "{stem}: the file prints its encode over a source exactly where the \
                 printer lowers every form ({forms:?})"
            ),
            // A codec of MCU-only features is generated on no host backend,
            // Kotlin among them, whatever its forms; one the printer prints
            // generates otherwise.
            Err(refusal) => assert!(
                !forms.is_empty() || refusal.contains("CodegenMcuClassKindOnNonMcuLanguage"),
                "{stem}: a codec the printer prints generates on Kotlin: {refusal}"
            ),
        }
        if forms.is_empty() {
            printed.push(stem);
        }
    }
    eprintln!("{} codec(s) printed on Kotlin:", printed.len());
    for stem in &printed {
        eprintln!("  {stem}");
    }
    for name in [
        "codec_length_ref",
        "codec_scout_zid_body",
        "codec_simple_frame",
        "broadcast_log_conformance_record",
        "codec_present_if_checked",
        "codec_length_shared",
        "codec_encoded_length_checked",
        "codec_length_stated",
        "codec_length_ref_dotted_basic",
        "codec_chain_handed_entry",
    ] {
        assert!(
            printed.iter().any(|stem| stem == name),
            "{name} is printed on Kotlin"
        );
    }
}
