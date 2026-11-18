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

mod common;

use std::path::{Path, PathBuf};

use sce_build::forge::error::{ForgeError, GenerateError};
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
            // The decode makes the bytes it read by the factory, a length its
            // class does not hold refused by the kind its guard refuses it
            // with (docs/adr/0099, amendment of 2026-11-16).
            "                    payload = CodecLengthRefPayloadBytes.of(payload) ?: return \
             Decoded.Refused(CodecError.LengthOutOfRange)\n",
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
            "                    locator = CodecZenohLocatorLocatorText.of(locator) ?: return \
             Decoded.Refused(CodecError.LengthOutOfRange)",
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

/// A decode of a codec whose byte string ends where its length says advances
/// the cursor past those bytes and no further, so a codec holding it as a body
/// or a chain's entry decodes on after it (`codec_origin_envelope`,
/// `codec_tlv_chain_basic`); a codec with a tail field consumes the frame
/// whole, the tail running to its end.
///
/// Turns red where the positional decode of `codec.kt.jinja2` advances by the
/// cursor's whole remainder without a tail field, or by less than it with one.
#[test]
fn a_decode_consumes_its_own_bytes_and_a_tail_the_rest_of_the_frame() {
    let leaf = resource_file("codec_origin_leaf");
    holds(
        "codec_origin_leaf",
        &leaf,
        &["            var _consumed = 1\n\
           \x20           if (1 + len.toInt() > _consumed) _consumed = 1 + len.toInt()\n\
           \x20           if (!cursor.advance(_consumed)) return Decoded.Refused(CodecError.NeedMoreBytes)\n"],
    );
    holds_none("codec_origin_leaf", &leaf, &["cursor.advance(frameLen)"]);
    let tail = resource_file("codec_tail");
    holds(
        "codec_tail",
        &tail,
        &["            if (!cursor.advance(frameLen)) return Decoded.Refused(CodecError.NeedMoreBytes)\n"],
    );
    holds_none("codec_tail", &tail, &["_consumed"]);
}

/// The Kotlin file `name`, the codec document `scxml` generates.
fn generated_file(name: &str, scxml: &str) -> String {
    sce_build::compile_forge_from_string(
        scxml,
        sce_build::DocumentLabel::symmetric(name),
        Language::Kotlin,
    )
    .unwrap_or_else(|e| panic!("{name} generates on Kotlin: {e:?}"))
    .files
    .into_iter()
    .find(|(file, _)| file.ends_with(".kt"))
    .map(|(_, text)| text)
    .unwrap_or_else(|| panic!("{name} generates a Kotlin file"))
}

/// A signed field is decoded as the signed type of its width, the type its
/// property is: a byte as the `Byte` a `ByteArray` holds, a word as its bits
/// taken two's complement, and not the unsigned number of its bits, which
/// kotlinc refuses where the property is signed. This holds where the frame is
/// read at fixed places and where it is read field by field. The encode writes
/// a signed byte as the `Byte` it is, and takes the bytes of a signed word of
/// four bytes off the `Int` it is: a conversion to the type a value has is one
/// kotlinc calls redundant, which `-Werror` refuses.
///
/// Turns red where `generate_decode_expr` or `streaming_fixed_field_body`
/// reads a signed byte through `toUByte()`, `decode_multibyte_unified`
/// converts a signed word to its unsigned type, `kotlin_byte_written`
/// converts a signed byte to `Byte`, or `encode_multibyte_unified` converts an
/// `Int` to `Int`.
#[test]
fn a_signed_field_is_decoded_as_the_signed_type_of_its_width() {
    const PLACED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="signed_placed" version="1.0">
  <datamodel>
    <sce:field id="small" sce:type="int8" sce:byte="0" sce:bit-size="8"/>
    <sce:field id="wide" sce:type="int16" sce:byte="1" sce:bit-size="16"/>
    <sce:field id="word" sce:type="int32" sce:byte="3" sce:bit-size="32"/>
  </datamodel>
</scxml>"#;
    let placed = generated_file("signed_placed", PLACED);
    holds(
        "signed_placed",
        &placed,
        &[
            "            val small = raw[0]\n",
            "            val wide = (((raw[1].toInt() and 0xFF) shl 8) or \
             (raw[2].toInt() and 0xFF)).toShort()\n",
            "w.writeU8(_e_small)?.let { return it }\n",
            // A `Short` has no shift: its bytes are taken off its `Int`.
            "w.writeU8((_e_wide.toInt() ushr 8 and 0xFF).toByte())?.let { return it }\n",
            "w.writeU8((_e_word ushr 24 and 0xFF).toByte())?.let { return it }\n",
        ],
    );
    holds_none(
        "signed_placed",
        &placed,
        &["raw[0].toUByte()", "_e_small.toByte()", "_e_word.toInt()"],
    );
    const STREAMED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="signed_streamed" version="1.0">
  <datamodel>
    <sce:flags id="flags" sce:type="uint8" sce:byte="0" sce:bit-size="8">
      <sce:flag name="has_late" bit="0"/>
    </sce:flags>
    <sce:field id="small" sce:type="int8" sce:byte="1" sce:bit-size="8"/>
    <sce:field id="late" sce:type="int16" sce:byte="2" sce:bit-size="16"
               sce:present-if="flags.has_late"/>
  </datamodel>
</scxml>"#;
    let streamed = generated_file("signed_streamed", STREAMED);
    holds(
        "signed_streamed",
        &streamed,
        &[
            "val _v = raw[0]\n",
            "val _v = (((raw[0].toInt() and 0xFF) shl 8) or \
             (raw[1].toInt() and 0xFF)).toShort()\n",
        ],
    );
}

/// The fold of the eight bytes from `at`, big-endian, each widened to a `Long`
/// before it is shifted.
fn long_fold(at: u32) -> String {
    (0..8)
        .map(|i| {
            let byte = format!("(raw[{}].toLong() and 0xFFL)", at + i);
            match 56 - 8 * i {
                0 => byte,
                up => format!("({byte} shl {up})"),
            }
        })
        .collect::<Vec<_>>()
        .join(" or ")
}

/// A field of eight bytes is folded in a `Long`, each byte widened before it
/// is shifted: Kotlin masks an `Int` shift count to five bits, so a byte of an
/// `Int` shifted by 32 or more lands in the low bits and every value read is
/// wrong. An unsigned field is the `ULong` of the fold and a signed one the
/// fold itself, where the frame is read at fixed places and field by field.
///
/// Turns red where `decode_multibyte_unified` folds more than four bytes in
/// `Int`s.
#[test]
fn a_field_of_eight_bytes_is_folded_in_a_long() {
    const PLACED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="wide_placed" version="1.0">
  <datamodel>
    <sce:field id="wide" sce:type="uint64" sce:byte="0" sce:bit-size="64"/>
    <sce:field id="signed_wide" sce:type="int64" sce:byte="8" sce:bit-size="64"/>
  </datamodel>
</scxml>"#;
    let placed = generated_file("wide_placed", PLACED);
    let unsigned = format!("            val wide = ({}).toULong()\n", long_fold(0));
    let signed = format!("            val signed_wide = ({})\n", long_fold(8));
    holds(
        "wide_placed",
        &placed,
        &[unsigned.as_str(), signed.as_str()],
    );
    holds_none("wide_placed", &placed, &["toInt() and 0xFF) shl 56"]);
    const STREAMED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="wide_streamed" version="1.0">
  <datamodel>
    <sce:flags id="flags" sce:type="uint8" sce:byte="0" sce:bit-size="8">
      <sce:flag name="has_late" bit="0"/>
    </sce:flags>
    <sce:field id="late" sce:type="uint64" sce:byte="1" sce:bit-size="64"
               sce:present-if="flags.has_late"/>
  </datamodel>
</scxml>"#;
    let streamed = generated_file("wide_streamed", STREAMED);
    let read = format!("val _v = ({}).toULong()\n", long_fold(0));
    holds("wide_streamed", &streamed, &[read.as_str()]);
    holds_none("wide_streamed", &streamed, &["toInt() and 0xFF) shl 56"]);
}

/// A bound past what an `Int` holds, the 2^32 - 1 bytes a Zenoh slice is
/// bounded by, is named by a `Long`, which kotlinc takes the literal as, and a
/// size is compared with it as with any other; a bound an `Int` holds is still
/// named by an `Int`.
///
/// Turns red where `kotlin_source::most_const` declares every most an `Int`,
/// which kotlinc refuses for this literal, or every most a `Long`.
#[test]
fn a_most_past_what_an_int_holds_is_named_by_a_long() {
    assert!(unlowered("codec_zenoh_ext_zbuf").is_empty());
    let text = resource_file("codec_zenoh_ext_zbuf");
    holds(
        "codec_zenoh_ext_zbuf",
        &text,
        &[
            "        const val MOST: Long = 4294967295L\n",
            "if (bytes.size <= MOST) CodecZenohExtZbufValueBytes(bytes) else null\n",
        ],
    );
    holds_none(
        "codec_zenoh_ext_zbuf",
        &text,
        &["const val MOST: Int = 4294967295"],
    );
    holds(
        "codec_repeat_basic",
        &resource_file("codec_repeat_basic"),
        &["        const val MOST: Int = 32\n"],
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
             val entries: List<CodecRepeatElemSource>) : com.sce.forge.runtime.Counted {\n",
            "    override val size: Int\n        get() = entries.size\n",
            "        const val MOST: Int = 32\n",
            "if (entries.size <= MOST) CodecRepeatBasicFragsList(entries.toList()) else null\n",
            "    val frags: CodecRepeatBasicFragsList\n",
            "    override var frags: CodecRepeatBasicFragsList = CodecRepeatBasicFragsList.fewest()",
            "val _d_num_frags = _e_frags.entries.size.toUByte()\n",
            "for (_entry in _e_frags.entries) {\n",
            "CodecRepeatElem.encodeSource(_entry, w)?.let { return it }\n",
            "frags = CodecRepeatBasicFragsList.of(frags) ?: return \
             Decoded.Refused(CodecError.TooManyElements)",
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
/// hands on.
///
/// Turns red where `kotlin_source::encode_source` drops `plan.given` from its
/// parameters or `gate_test` stops spelling a `Clause::Input`, or where the
/// template's `encode` stops handing its inputs on.
#[test]
fn a_flag_input_the_value_does_not_say_is_handed_to_the_encode() {
    assert!(unlowered("codec_chain_handed_entry").is_empty());
    let text = resource_file("codec_chain_handed_entry");
    holds(
        "codec_chain_handed_entry",
        &text,
        &[
            "        fun encodeSource(src: CodecChainHandedEntrySource, w: SceSink, \
             afterMarker: UByte, overrides: CodecChainHandedEntryOverrides? = null): \
             CodecError? {\n",
            "            if ((afterMarker.toUInt() != 0u || ((_c_header.toUInt() shr 6) and 0x1u) \
             != 0x0u) != (_e_value != null)) {\n\
             \x20               return CodecError.PresentIfMismatch\n",
            "        return encodeSource(this, w, afterMarker)\n",
        ],
    );
}

/// A codec that presents one-bit flags of a fixed carrier lets a parent set
/// them in place of the value's (`EncodePlan::overridable`), and a chain sets
/// on each entry but the last the flag its entries end on and hands each entry
/// whether the entry before it was of the kind its input waits for.
///
/// Turns red where `kotlin_source::overrides_declaration` stops declaring the
/// class, `encode_source` stops folding an override into its carrier, or the
/// walk of a chain stops counting the entries left or keeping the entry before
/// (`ListWrite.ends`, `ListWrite.previous`), or `positional_forms` names such a
/// chain unlowered.
#[test]
fn a_chain_sets_the_flag_its_entries_end_on_and_hands_on_the_entry_before() {
    let entry = "codec_chain_handed_entry";
    let text = resource_file(entry);
    holds(
        entry,
        &text,
        &[
            "class CodecChainHandedEntryOverrides(\n",
            "    val header_wide: Boolean? = null,\n",
            "    val header_Z: Boolean? = null,\n)\n",
            "            overrides?.header_Z?.let { _u_header = (_u_header and 0x80u.inv()) \
             or (if (it) 0x80u else 0x0u) }\n",
        ],
    );
    let chain = "codec_chain_handed";
    assert!(
        unlowered(chain).is_empty(),
        "{chain}: {:?}",
        unlowered(chain)
    );
    let text = resource_file(chain);
    holds(
        chain,
        &text,
        &[
            "            var _prev_afterMarker: UByte = 0u\n",
            "            var _left_entries = _e_entries.entries.size\n",
            "                _left_entries -= 1\n",
            "CodecChainHandedEntry.encodeSource(_entry, ",
            ", _prev_afterMarker, CodecChainHandedEntryOverrides(header_Z = \
             _left_entries != 0))?.let { return it }\n",
            "                _prev_afterMarker = (if (((_entry.header.toULong() shr 0) and 0xFuL) \
             == 0x4uL) 1 else 0).toUByte()\n",
        ],
    );
}

/// A chain closed by its entries' flag holds one entry at least (docs/adr/0065
/// §6 R7): the class of its bound names `LEAST`, makes no value of fewer
/// entries, and a value made by default holds that many entries, each its
/// entry codec's value made by default.
///
/// Turns red where `kotlin_source::value_forms` refuses a list of a least past
/// zero again, `ListBound::of` leaves the least out of the test, or
/// `ListBound::fewest` makes an empty list for a least past zero.
#[test]
fn a_chain_closed_by_its_entries_flag_holds_one_entry_at_least() {
    let name = "codec_chain_handed";
    assert!(unlowered(name).is_empty(), "{name}: {:?}", unlowered(name));
    let text = resource_file(name);
    holds(
        name,
        &text,
        &[
            "        const val LEAST: Int = 1\n",
            "if (entries.size in LEAST..MOST) CodecChainHandedEntriesList(entries.toList()) \
             else null\n",
            "        fun fewest(): CodecChainHandedEntriesList = \
             CodecChainHandedEntriesList(List(LEAST) { CodecChainHandedEntry() })\n",
        ],
    );
    holds_none(name, &text, &["CodecChainHandedEntriesList(emptyList())"]);
}

/// A gate group (docs/adr/0065 §6 R4b) is one property of the value: a sealed
/// class of a data class for each arm, each holding the members present while
/// the gate holds its value, the stated length in the arm without its content.
/// The encode reads each member off the arm the value holds, `null` where it
/// holds none, derives the flag and the length from them, and the decode makes
/// the arm its gate selects of what it read.
///
/// Turns red where `kotlin_source::positional_forms` names a group of a flag
/// unlowered, `group::declare` stops nesting the arms in the sealed class or
/// typing a member as the class of its bound, `group::member_read` reads a
/// member other than off the arm the value holds, `derive_number` stops reading
/// the stated length off its arm (`Absent::Group`), `derived_field` keeps a
/// member a field of the data class, or the template stops making the group.
#[test]
fn a_group_is_one_property_whose_arms_hold_its_members() {
    let name = "codec_present_if_length_ref";
    assert!(unlowered(name).is_empty(), "{name}: {:?}", unlowered(name));
    let text = resource_file(name);
    holds(
        name,
        &text,
        &[
            "sealed class CodecPresentIfLengthRefFlagsHasPayload {\n",
            "    data class HasPayload(\n        val payload: CodecPresentIfLengthRefPayloadBytes \
             = CodecPresentIfLengthRefPayloadBytes.fewest()\n    ) : \
             CodecPresentIfLengthRefFlagsHasPayload()\n",
            "    data class NotHasPayload(\n        val payload_size: UByte = 0.toUByte()\n    ) : \
             CodecPresentIfLengthRefFlagsHasPayload()\n",
            "    val flags_has_payload: CodecPresentIfLengthRefFlagsHasPayload\n",
            // A value made by default holds the arm present while no gate
            // holds, as the Rust, C++ and Python values do.
            "    override var flags_has_payload: CodecPresentIfLengthRefFlagsHasPayload = \
             CodecPresentIfLengthRefFlagsHasPayload.NotHasPayload()\n",
            "            val _g_flags_has_payload = src.flags_has_payload\n",
            "            val _e_payload = when (val _g = _g_flags_has_payload) { \
             is CodecPresentIfLengthRefFlagsHasPayload.HasPayload -> _g.payload; else -> null }\n",
            "            val _e_payload_size = when (val _g = _g_flags_has_payload) { \
             is CodecPresentIfLengthRefFlagsHasPayload.NotHasPayload -> _g.payload_size; \
             else -> null }\n",
            "            val _d_payload_size = if (_e_payload != null) \
             _e_payload.bytes.size.toUByte() else (_e_payload_size ?: 0.toUByte())\n",
            "                    flags_has_payload = if (((flags.toUInt() shr 0) and 0x1u) != 0x0u) \
             CodecPresentIfLengthRefFlagsHasPayload.HasPayload(payload = \
             CodecPresentIfLengthRefPayloadBytes.of(payload!!) ?: return \
             Decoded.Refused(CodecError.LengthOutOfRange)) else \
             CodecPresentIfLengthRefFlagsHasPayload.NotHasPayload(payload_size = payload_size)\n",
        ],
    );
    holds_none(
        name,
        &text,
        &["var payload: ", "var payload_size: ", "src.payload"],
    );
}

/// The flags of a disjunction that decides a group (docs/adr/0065 §6 R4c) are
/// each derived from the arm the value holds, and the value may hold none.
///
/// Turns red where `kotlin_source::encode_source` stops folding a
/// `Derive::ArmFlag` from the group, or `group::gate_holds` tests other than
/// the arms that set the gate.
#[test]
fn a_disjunctions_flags_are_derived_from_the_arm_the_value_holds() {
    let name = "codec_present_if_disjunction";
    assert!(unlowered(name).is_empty(), "{name}: {:?}", unlowered(name));
    let text = resource_file(name);
    holds(
        name,
        &text,
        &[
            "    val flags_wants_a_or_wants_b: CodecPresentIfDisjunctionFlagsWantsAOrWantsB?\n",
            "            val _g_flags_wants_a_or_wants_b = src.flags_wants_a_or_wants_b\n",
            "_u_flags = (_u_flags and 0x1u.inv()) or (if ((_g_flags_wants_a_or_wants_b is \
             CodecPresentIfDisjunctionFlagsWantsAOrWantsB.",
            "_u_flags = (_u_flags and 0x2u.inv()) or (if ((_g_flags_wants_a_or_wants_b is \
             CodecPresentIfDisjunctionFlagsWantsAOrWantsB.",
            " else null\n",
        ],
    );
}

/// Two lists of one count (docs/adr/0065 §6 R5) are one member of the arm that
/// holds both, the runtime's `Aligned` of the two, which makes no value of two
/// lists of different counts; the encode derives the count from the first and
/// reads each list out of the pair, and the decode pairs what it read.
///
/// Turns red where `kotlin_source::positional_forms` names two lists of one
/// count unlowered, `group::declare` holds the second list as a member of its
/// own, `group::member_read` reads a list other than out of the pair, or
/// `bounded_list_class` stops implementing the runtime's `Counted`.
#[test]
fn two_lists_of_one_count_are_one_aligned_member_of_their_arm() {
    let name = "codec_repeat_unconditional_count";
    assert!(unlowered(name).is_empty(), "{name}: {:?}", unlowered(name));
    let text = resource_file(name);
    holds(
        name,
        &text,
        &[
            "        val links_weights: com.sce.forge.runtime.Aligned<\
             CodecRepeatUnconditionalCountLinksList, CodecRepeatUnconditionalCountWeightsList> \
             = com.sce.forge.runtime.Aligned.of(CodecRepeatUnconditionalCountLinksList.fewest(), \
             CodecRepeatUnconditionalCountWeightsList.fewest())!!\n",
            " -> _g.links_weights.first; ",
            " -> _g.links_weights.second; else -> null }\n",
            "            val _d_links_len = _e_links.entries.size",
            // Each list past its bound is refused by its kind; two lists read
            // under one count always pair.
            "links_weights = com.sce.forge.runtime.Aligned.of(\
             CodecRepeatUnconditionalCountLinksList.of(links) ?: return \
             Decoded.Refused(CodecError.TooManyElements), \
             CodecRepeatUnconditionalCountWeightsList.of(weights!!) ?: return \
             Decoded.Refused(CodecError.TooManyElements)) \
             ?: error(\"two lists read under one count hold as many entries\")",
        ],
    );
    holds_none(name, &text, &["var weights: ", "var links: "]);
}

/// An input a group decides (docs/adr/0065 §6 R4b) is said by the value: set
/// where it holds the group's arm, and not handed to its encode.
///
/// Turns red where `kotlin_source::positional_forms` names an input a group
/// says unlowered, or `said_value` reads it other than off the group.
#[test]
fn an_input_a_group_says_is_read_off_the_arm_the_value_holds() {
    let name = "codec_zenoh_init_body";
    assert!(unlowered(name).is_empty(), "{name}: {:?}", unlowered(name));
    let text = resource_file(name);
    holds(
        name,
        &text,
        &[
            "        fun inputS(src: CodecZenohInitBodySource): UByte =\n            \
             (if (src.s != null) 1 else 0).toUByte()",
            "    val s: CodecZenohInitBodyS?\n",
            "            val _e_sn_res = _g_s?.sn_res\n",
        ],
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
            // The input is the parameter every Kotlin site names it by, its
            // declared spelling (`compute_flag_input_param_fragments`).
            "        if (S != inputS(this)) return CodecError.PresentIfMismatch\n\
             \x20       return encodeSource(this, w)\n",
            "    fun encodeToByteArray(S: UByte): ByteArray? {\n",
        ],
    );
}

/// A variant is presented as the arm the value holds, each arm a source of its
/// codec: the encode derives the tag field from the arm, the catch-all's tag
/// read off the class of the tags no arm names, and writes the arm by its
/// codec's own encode; the decode makes the catch-all's tag by that class's
/// factory.
///
/// Turns red where `kotlin_source::positional_forms` names a variant of
/// printed arms unlowered, `arm_tag` reads the tag other than off the arm,
/// `derived_fields` leaves the tag field to the value, `render` declares no
/// class of the catch-all's tag, or the template holds an arm as its data
/// class rather than its source.
#[test]
fn a_variant_derives_its_tag_from_the_arm_and_writes_the_arm_by_its_codec() {
    assert!(unlowered("codec_variant_dispatch").is_empty());
    let text = resource_file("codec_variant_dispatch");
    holds(
        "codec_variant_dispatch",
        &text,
        &[
            "interface CodecVariantDispatchSource {\n    val body: CodecVariantDispatchVariant\n}\n",
            "    data class CodecVariantSessionOpen(val body: \
             com.sce.generated.codec_variant_session_open.CodecVariantSessionOpenSource)",
            "    data class Default(val tag: CodecVariantDispatchUnknownTag, val body: \
             com.sce.generated.codec_variant_session_close.CodecVariantSessionCloseSource)",
            "value class CodecVariantDispatchUnknownTag private constructor(val tag: UByte) {\n",
            "                in 0x1u..0x2u -> null\n",
            "            val _r_body = src.body\n",
            "            val _t_body: ULong = when (val _b = _r_body) {\n\
             \x20               is CodecVariantDispatchVariant.CodecVariantSessionOpen -> 0x1uL\n\
             \x20               is CodecVariantDispatchVariant.CodecVariantSessionClose -> 0x2uL\n\
             \x20               is CodecVariantDispatchVariant.Default -> _b.tag.wide\n\
             \x20           }\n",
            "            val _d_msg_id = _t_body.toUByte()\n",
            "                is CodecVariantDispatchVariant.CodecVariantSessionOpen -> \
             CodecVariantSessionOpen.encodeSource(_b.body, w)?.let { return it }\n",
            // The catch-all is reached only under a tag no arm names, which
            // its class holds: no frame is refused there.
            "CodecVariantDispatchUnknownTag.of(msg_id) ?: error(\"a tag no arm names is one \
             CodecVariantDispatchUnknownTag holds\")",
            "    override var body: CodecVariantDispatchVariant = ",
        ],
    );
    holds_none("codec_variant_dispatch", &text, &["var msg_id: UByte"]);
}

/// A parent whose carrier's bits the arm of an embedded body's variant fills
/// derives them from the arm the body holds, read through the body's source.
///
/// Turns red where `kotlin_source::encode_source` reads the parent tag other
/// than off the arm of the body's variant, or folds it into the carrier other
/// than in place of the bits the value presents.
#[test]
fn a_parent_tag_is_derived_from_the_arm_its_body_holds() {
    assert!(unlowered("codec_source_parent_tag").is_empty());
    let text = resource_file("codec_source_parent_tag");
    holds(
        "codec_source_parent_tag",
        &text,
        &[
            "            var _p_hdr_kind: ULong = 0x0uL\n\
             \x20           when (_e_body.body) {\n\
             \x20               is CodecSourceLeafVariant.CodecVariantSessionOpen -> \
             _p_hdr_kind = _p_hdr_kind or 0x0uL\n\
             \x20               is CodecSourceLeafVariant.CodecVariantSessionClose -> \
             _p_hdr_kind = _p_hdr_kind or 0x1uL\n\
             \x20           }\n",
            "_u_hdr = (_u_hdr and 0x1u.inv()) or ((_p_hdr_kind and 0x1uL).toUInt() shl 0)\n",
        ],
    );
}

/// An input the arms of a variant say is read off the arm the value holds, by
/// that arm's codec's function (docs/adr/0065 §6 R4d); where some arms do not
/// read it, the value says nothing from them, `null` (R4e), and so does a codec
/// whose body says it so. A parent holds what such a body says to what it hands
/// it only where the body says anything.
///
/// Turns red where `kotlin_source::said_value` reads an arm's input other than
/// off the arm, answers a number from an arm that does not read it, the said
/// function is typed `UByte` where the value may say nothing, `render` refuses
/// a handed input a value that says nothing of it, the check of a body that
/// says it in some values only refuses one that says nothing, or the check
/// converts the `UInt` flag of a byte's carrier to `UInt` again.
#[test]
fn an_input_the_arms_say_is_read_off_the_arm_the_value_holds() {
    for name in [
        "codec_every_arm_says",
        "codec_every_arm_says_parent",
        "codec_some_arms_say",
        "codec_some_arms_say_forward",
        "codec_some_arms_say_parent",
    ] {
        assert!(unlowered(name).is_empty(), "{name}: {:?}", unlowered(name));
    }
    let every = resource_file("codec_every_arm_says");
    holds(
        "codec_every_arm_says",
        &every,
        &["        fun inputN(src: CodecEveryArmSaysSource): UByte =\n            \
           when (val _b = src.body) { \
           is CodecEveryArmSaysVariant.CodecEveryArmSaysFar -> CodecEveryArmSaysFar.inputBare(_b.body); \
           is CodecEveryArmSaysVariant.CodecEveryArmSaysNear -> CodecEveryArmSaysNear.inputN(_b.body) }"],
    );
    let some = resource_file("codec_some_arms_say");
    holds(
        "codec_some_arms_say",
        &some,
        &[
            "        fun inputN(src: CodecSomeArmsSaySource): UByte? =\n            \
             when (val _b = src.body) { \
             is CodecSomeArmsSayVariant.CodecSomeArmsSayPlain -> null; \
             is CodecSomeArmsSayVariant.CodecEveryArmSaysNear -> CodecEveryArmSaysNear.inputN(_b.body) }",
            "        if (inputN(this).let { it != null && it != N }) return \
             CodecError.PresentIfMismatch\n",
        ],
    );
    let forward = resource_file("codec_some_arms_say_forward");
    holds(
        "codec_some_arms_say_forward",
        &forward,
        &[
            "        fun inputN(src: CodecSomeArmsSayForwardSource): UByte? =\n            \
           CodecSomeArmsSay.inputN(src.body)",
        ],
    );
    let parent = resource_file("codec_some_arms_say_parent");
    holds(
        "codec_some_arms_say_parent",
        &parent,
        &[
            "            if (CodecSomeArmsSay.inputN(_e_body).let { it != null && it.toUInt() != ",
            // The flag of a byte's carrier is a `UInt` already, which a
            // second `toUInt()` kotlinc calls redundant would only repeat.
            "it.toUInt() != ((_c_header.toUInt() shr 5) and 0x1u) }) {\n",
        ],
    );
    holds_none(
        "codec_some_arms_say_parent",
        &parent,
        &["and 0x1u).toUInt()"],
    );
}

/// A group of a chain's memberships (docs/adr/0065 §6 R6b) holds the chain in
/// every arm, each in the class of the kinds the arm says it holds an entry of
/// and none of, made only of entries that do; an arm that says it holds no
/// kind of a gated chain may hold none. The encode reads the one list of
/// entries every class holds and tests a gate on a membership by the arm the
/// value holds; the decode selects the arm by the kinds of the chain it read.
///
/// Turns red where `kotlin_source::positional_forms` names a group of a
/// chain's memberships unlowered, `group::chain_declaration` stops testing a
/// kind the arm holds or lacks, `group::declare` types the chain of an arm
/// that holds no kind of a gated chain as always there, `group::member_read`
/// reads the chain other than as its entries, `gate_test` stops reading a
/// membership off the arm the value holds, or `group::decoded_test` stops
/// testing the chain the decode read.
#[test]
fn a_group_of_a_chains_memberships_holds_the_chain_in_the_class_of_its_kinds() {
    let name = "codec_chain_has_envelope";
    assert!(unlowered(name).is_empty(), "{name}: {:?}", unlowered(name));
    let text = resource_file(name);
    holds(
        name,
        &text,
        &[
            "value class CodecChainHasEnvelopeExtensionsHas2Chain private constructor(\
             val entries: List<CodecZenohExtEntrySource>) : com.sce.forge.runtime.Counted {\n",
            "        private fun kind(entry: CodecZenohExtEntrySource): ULong = \
             ((entry.header.toULong() shr 0) and 0xFuL)\n",
            "if (entries.size in LEAST..MOST && entries.any { kind(it) == 0x2uL }) \
             CodecChainHasEnvelopeExtensionsHas2Chain(entries.toList()) else null\n",
            "if (entries.size in LEAST..MOST && entries.none { kind(it) == 0x2uL }) \
             CodecChainHasEnvelopeExtensionsLacks2Chain(entries.toList()) else null\n",
            "        val extensions: CodecChainHasEnvelopeExtensionsHas2Chain,\n",
            "        val extensions: CodecChainHasEnvelopeExtensionsLacks2Chain? = null,\n",
            " -> _g.extensions.entries; is ",
            " -> _g.extensions?.entries }\n",
            "if (!(_g_extensions_has_0x2 is CodecChainHasEnvelopeExtensionsHas0x2.Has0x2)) {\n",
            // A chain read that disagrees with the gate that selects the arm
            // is refused as a gate given where its predicate does not hold.
            // The test that selects the arm proves the gated chain was read,
            // so it is read with no assertion, which kotlinc calls needless.
            "if (_has_extensions_2) CodecChainHasEnvelopeExtensionsHas0x2.Has0x2(extensions = \
             CodecChainHasEnvelopeExtensionsHas2Chain.of(extensions) ?: return \
             Decoded.Refused(CodecError.PresentIfMismatch)",
            "extensions = extensions?.let { CodecChainHasEnvelopeExtensionsLacks2Chain.of(it) \
             ?: return Decoded.Refused(CodecError.PresentIfMismatch) }",
        ],
    );
    // An arm that says the chain holds a kind has no chain made by default.
    holds_none(
        name,
        &text,
        &["fun fewest(): CodecChainHasEnvelopeExtensionsHas2Chain"],
    );
}

/// A value made by default holds the arm of a group of a chain's memberships
/// where no gate holds, whose chain is made of the fewest entries its bound
/// holds, each of no kind it lacks, and a flag joined with the memberships is
/// derived from the arm the value holds.
///
/// Turns red where `group::declare` makes the first arm the default, where
/// `group::chain_declaration` gives a class whose arm holds a kind a `fewest`,
/// or where `encode_source` stops folding a flag of the group from its arm.
#[test]
fn a_value_made_by_default_holds_the_arm_where_no_membership_holds() {
    let name = "codec_chain_has_tagged";
    assert!(unlowered(name).is_empty(), "{name}: {:?}", unlowered(name));
    let text = resource_file(name);
    holds(
        name,
        &text,
        &[
            "    override var entries_has_0x7_has_0x9_header_wide: \
             CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide = \
             CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide.Neither()",
            "        val entries: CodecChainHasTaggedEntriesLacks7Lacks9Chain = \
             CodecChainHasTaggedEntriesLacks7Lacks9Chain.fewest(),\n",
            "        fun fewest(): CodecChainHasTaggedEntriesLacks7Lacks9Chain =\n            \
             checkNotNull(of(List(LEAST) { CodecChainHasTaggedEntry() })) \
             { \"a value-made entry is of no kind the chain lacks\" }\n",
            "        private fun kind(entry: CodecChainHasTaggedEntrySource): ULong = \
             entry.entry_type.toULong()\n",
            "entries.any { kind(it) == 0x7uL } && entries.any { kind(it) == 0x9uL }",
            "_u_header = (_u_header and 0x1u.inv()) or (if ((_g_entries_has_0x7_has_0x9_header_wide \
             is CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide.Has0x7Has0x9Wide",
        ],
    );
    holds_none(
        name,
        &text,
        &["fun fewest(): CodecChainHasTaggedEntriesHas7Has9Chain"],
    );
}

/// A gate on a chain's kinds joined with a flag input is a check (docs/adr/0065
/// §6 R4c, R6): the value is held to it before the first byte, the membership
/// a scan of the chain's entries, each told apart by the chain's `entry-id`.
///
/// Turns red where `kotlin_source::encode_source`'s `gate_test` stops scanning
/// the entries for a membership no group holds.
#[test]
fn a_membership_no_group_holds_is_a_scan_of_the_chains_entries() {
    let name = "codec_chain_has_input";
    assert!(unlowered(name).is_empty(), "{name}: {:?}", unlowered(name));
    let text = resource_file(name);
    holds(
        name,
        &text,
        &["            if ((_e_entries.entries.any { _x -> _x.entry_type.toULong() == 0x7uL } || \
           urgent.toUInt() != 0u) != (_e_priority != null)) {\n\
           \x20               return CodecError.PresentIfMismatch\n"],
    );
}

/// The arm where the one gate on a bare entry holds (docs/adr/0065 §6 R6a)
/// holds the other entries and the named entry's place, made only where no
/// other entry before the place is of the kind. The encode walks the chain as
/// slots, writing the entry of the kind, made of its identifier and the empty
/// arm its tag selects, where the value holds its place; the decode takes the
/// first entry of the kind out of the chain it read.
///
/// Turns red where `kotlin_source::positional_forms` refuses a chain of slots,
/// `slots_walk` stops writing the named entry at its place, `named_entry`
/// makes the entry other than of its identifier and the arm its tag selects,
/// `group::placed_declaration` stops holding the place to the first of its
/// kind, `group::member_read` reads other than the others of the place, or
/// the decode stops taking the place by `placing`.
#[test]
fn a_bare_entrys_arm_holds_the_others_and_the_named_entrys_place() {
    let name = "codec_chain_has_marker";
    assert!(unlowered(name).is_empty(), "{name}: {:?}", unlowered(name));
    let text = resource_file(name);
    holds(
        name,
        &text,
        &[
            "class CodecChainHasMarkerExtensionsHas12Placed private constructor(\n",
            "    val rest: List<CodecZenohExtEntrySource>,\n",
            "if (rest.size < MOST && place in 0..rest.size && \
             rest.take(place).none { kind(it) == 0x12uL }) \
             CodecChainHasMarkerExtensionsHas12Placed(rest.toList(), place) else null\n",
            "        fun named(): CodecZenohExtEntry = CodecZenohExtEntry(header = \
             0x12u.toUByte(), body = CodecZenohExtEntryVariant.CodecZenohExtUnit(\
             com.sce.generated.codec_zenoh_ext_unit.CodecZenohExtUnit()))\n",
            "        val extensions: CodecChainHasMarkerExtensionsHas12Placed,\n",
            " -> _g.extensions.rest; is ",
            "val _p_extensions = when (val _g = _g_extensions_has_0x12) { \
             is CodecChainHasMarkerExtensionsHas0x12.Has0x12 -> _g.extensions.place; \
             else -> null }\n",
            "val _k_extensions = _e_extensions.size + (if (_p_extensions != null) 1 else 0)\n",
            "if (_i == _p_extensions) {\n",
            "encodeSource(CodecChainHasMarkerExtensionsHas12Placed.named(), ",
            "CodecZenohExtEntryOverrides(header_Z = _i + 1 < _k_extensions))?.let { return it }\n",
            "extensions = CodecChainHasMarkerExtensionsHas12Placed.placing(extensions) \
             ?: return Decoded.Refused(CodecError.PresentIfMismatch)",
        ],
    );
}

/// A list whose entries say the inputs it hands them is held in the class of
/// its law (docs/adr/0065 §6 R8): the list in the class of its bound and the
/// start of the law, made only of entries that follow the law from the start.
/// The encode derives the flag the list's start binds; the decode holds what
/// it read to the law from the start the frame gave.
///
/// Turns red where `kotlin_source::value_forms` refuses a list held to a law,
/// `lawful_declaration` stops holding an entry to the start or to the kind of
/// the entry before, `encode_source` derives the bound flag other than from
/// the list's start (`Measured::Start`), or `decoded_starts` starts the decode
/// other than from the carrier's bit.
#[test]
fn a_list_held_to_a_law_is_held_in_the_class_of_its_law() {
    let name = "codec_chain_prev_carrier";
    assert!(unlowered(name).is_empty(), "{name}: {:?}", unlowered(name));
    let text = resource_file(name);
    holds(
        name,
        &text,
        &[
            "class CodecChainPrevCarrierEntriesLawful private constructor(\n",
            "    val list: CodecChainPrevCarrierEntriesList,\n",
            "    val startTagged: Boolean,\n",
            "        fun of(list: CodecChainPrevCarrierEntriesList, startTagged: Boolean): \
             CodecChainPrevCarrierEntriesLawful? {\n",
            "            var afterAfterMarker = false\n",
            "                if ((CodecChainPrevEntry.inputTagged(entry).toUInt() != 0u) != \
             startTagged) return null\n",
            "                if ((CodecChainPrevEntry.inputAfterMarker(entry).toUInt() != 0u) != \
             afterAfterMarker) return null\n",
            "                afterAfterMarker = kind(entry) == 0x4uL\n",
            "    val entries: CodecChainPrevCarrierEntriesLawful\n",
            "_u_head = (_u_head and 0x1u.inv()) or \
             (((if (_e_entries.startTagged) 1 else 0)).toUInt() shl 0)\n",
            // A list past its bound is refused by its kind, and entries that
            // break the law from the start the frame gave as the Rust decode
            // refuses them.
            "entries = CodecChainPrevCarrierEntriesLawful.of(\
             CodecChainPrevCarrierEntriesList.of(entries) ?: return \
             Decoded.Refused(CodecError.TooManyElements), \
             startTagged = ((head.toUInt() shr 0) and 0x1u) != 0x0u) ?: return \
             Decoded.Refused(CodecError.PresentIfMismatch)",
        ],
    );
}

/// A CBOR map is printed from its own plan: each text and byte string in the
/// value class of its bound (an exact length a least equal to its most), an
/// integer as the unsigned type of its width, every entry a property of the
/// source interface, `null` for one the map may lack; the encode over any
/// source reads each entry once, counts the optional entries present, and
/// writes the head and each entry present in ascending key order.
///
/// Turns red where `kotlin_source::cbor::render` writes the entries in
/// declaration order rather than by `by_key`, counts an optional entry the
/// value lacks, holds a byte string in a bare `ByteArray`, refuses a decoded
/// entry with a kind other than `past_bound` names, or where `unlowered_forms`
/// names a CBOR map again.
#[test]
fn a_cbor_map_holds_each_entry_in_the_class_of_its_bound_and_writes_it_by_key() {
    assert!(unlowered("codec_cbor_map").is_empty());
    let text = resource_file("codec_cbor_map");
    holds(
        "codec_cbor_map",
        &text,
        &[
            "import kotlin.jvm.JvmInline\n",
            "@JvmInline\nvalue class CodecCborMapIdBytes private constructor(val bytes: ByteArray) {\n",
            "        const val LEAST: Int = 16\n",
            "        fun of(bytes: ByteArray): CodecCborMapIdBytes? =\n            \
             if (bytes.size in LEAST..MOST) CodecCborMapIdBytes(bytes) else null\n",
            "@JvmInline\nvalue class CodecCborMapSourceText private constructor(val text: String) {\n",
            "interface CodecCborMapSource {\n    val source: CodecCborMapSourceText\n    \
             val id: CodecCborMapIdBytes\n    val pattern: UByte\n    \
             val payload: CodecCborMapPayloadBytes\n    val deadline: ULong?\n    \
             val urgent: Boolean?\n}\n",
            "    override var id: CodecCborMapIdBytes = CodecCborMapIdBytes.fewest(),\n",
            "    override var deadline: ULong? = null,\n",
            ") : CodecCborMapSource {\n",
            "    fun encode(w: SceSink): CodecError? = encodeSource(this, w)\n",
            "        fun encodeSource(src: CodecCborMapSource, w: SceSink): CodecError? {\n",
            "            var count = 4UL\n            if (_e_deadline != null) count++\n            \
             if (_e_urgent != null) count++\n            \
             Cbor.writeMapHead(w, count)?.let { return it }\n            \
             Cbor.writeUint(w, 0UL)?.let { return it }\n            \
             Cbor.writeBytes(w, _e_id.bytes)?.let { return it }\n            \
             Cbor.writeUint(w, 1UL)?.let { return it }\n            \
             Cbor.writeText(w, _e_source.text)?.let { return it }\n            \
             Cbor.writeUint(w, 3UL)?.let { return it }\n            \
             Cbor.writeUint(w, _e_pattern.toULong())?.let { return it }\n",
            "            if (_e_urgent != null) {\n                \
             Cbor.writeUint(w, 7UL)?.let { return it }\n                \
             Cbor.writeBool(w, _e_urgent)?.let { return it }\n            }\n            \
             if (_e_deadline != null) {\n                \
             Cbor.writeUint(w, 12UL)?.let { return it }\n                \
             Cbor.writeUint(w, _e_deadline)?.let { return it }\n            }\n",
            // A required entry the map lacks is refused as a missing key, and
            // bytes of another length than the class holds as the reader
            // refuses them (docs/adr/0099, amendment of 2026-11-16).
            "                    id = CodecCborMapIdBytes.of(id ?: return \
             Decoded.Refused(CodecError.CborRequiredKeyMissing)) ?: return \
             Decoded.Refused(CodecError.CborWrongLength),\n",
            "                    payload = CodecCborMapPayloadBytes.of(payload ?: return \
             Decoded.Refused(CodecError.CborRequiredKeyMissing)) ?: return \
             Decoded.Refused(CodecError.CborOutOfRange),\n",
        ],
    );
    // The template's checks are gone: the classes hold every bound, and the
    // map refuses nothing. Neither the source interface nor the data class
    // holds the byte string bare; the decode alone reads it bare, into the
    // local its factory then judges (`id = CodecCborMapIdBytes.of(id ...)`).
    holds_none(
        "codec_cbor_map",
        &text,
        &[
            "return CodecError.CborWrongLength",
            "return CodecError.CborOutOfRange",
            "    val id: ByteArray",
            "override var id: ByteArray",
        ],
    );
}

/// A content-line component is printed from its own plan: every property and
/// each of its parameters a property of the source interface, nullable where it
/// may be absent, a repeated property a `List`; the encode over any source
/// writes the component into a staged sink first, judges a list against its
/// count before its lines, and refuses a parameter given without its property,
/// so a refusal leaves the caller's sink untouched.
///
/// Turns red where `kotlin_source::content_line::render` writes into the
/// caller's sink before the component is whole, drops the judgement of a
/// list's count, or reads an optional parameter without narrowing it, or where
/// `unlowered_forms` names a content-line component again.
#[test]
fn a_content_line_component_is_written_whole_into_a_staged_sink_first() {
    assert!(unlowered("codec_content_line_event").is_empty());
    let text = resource_file("codec_content_line_event");
    holds(
        "codec_content_line_event",
        &text,
        &[
            "interface CodecContentLineEventSource {\n    val uid: String\n    \
             val dtstart: String\n    val dtstartTzid: String\n",
            "    val exdate: List<String>\n    val sequence: UShort?\n",
            ") : CodecContentLineEventSource {\n",
            "    override var organizerCn: String? = null,\n",
            "    fun encode(w: SceSink): CodecError? = encodeSource(this, w)\n",
            "        fun encodeSource(src: CodecContentLineEventSource, w: SceSink): CodecError? {\n            \
             val _f = mutableListOf<Byte>()\n            \
             val out = ContentLineWriter(MutableListSink(_f), \"VEVENT\")\n            \
             out.begin()?.let { return it }\n",
            "            out.property(\"DTSTART\")?.let { return it }\n            \
             out.param(\"TZID\", src.dtstartTzid, 24)?.let { return it }\n",
            "            val _e_organizer = src.organizer\n            \
             val _p_organizerCn = src.organizerCn\n            \
             if (_e_organizer != null) {\n                \
             out.property(\"ORGANIZER\")?.let { return it }\n                \
             if (_p_organizerCn != null) out.param(\"CN\", _p_organizerCn, 24)?.let { return it }\n",
            "            } else if (_p_organizerCn != null) {\n                \
             return CodecError.LineRequiredMissing\n            }\n",
            "            if (_e_exdate.size > 3) return CodecError.LineTooMany\n            \
             for (_v in _e_exdate) {\n",
            "                out.uint(_e_sequence.toULong())?.let { return it }\n",
            "            out.finish()?.let { return it }\n            \
             return w.writeBytes(_f.toByteArray())\n        }\n",
        ],
    );
    // The template's encode, which wrote into the caller's sink as it went, is
    // gone.
    holds_none(
        "codec_content_line_event",
        &text,
        &[
            "val out = ContentLineWriter(w, ",
            "        run {\n            val v = ",
        ],
    );
}

/// A repeated property whose lines are records presents a `List` of the
/// record's source interface, which the record's data class implements; the
/// encode reads an optional parameter of each line into a local it narrows.
///
/// Turns red where `kotlin_source::content_line::render` declares no
/// interface of a line record, or where `write_params` reads a record's
/// optional parameter twice off the record.
#[test]
fn a_line_record_is_a_source_of_its_own_interface() {
    assert!(unlowered("codec_content_line_records").is_empty());
    let text = resource_file("codec_content_line_records");
    holds(
        "codec_content_line_records",
        &text,
        &[
            "interface CodecContentLineRecordsAttendeeLineSource {\n    \
             val attendeeCn: String?\n    val attendeeRole: String\n    val value: String\n}\n",
            "    val attendee: List<CodecContentLineRecordsAttendeeLineSource>\n",
            ") : CodecContentLineRecordsAttendeeLineSource\n",
            "            for (_line in _e_attendee) {\n                \
             out.property(\"ATTENDEE\")?.let { return it }\n                \
             val _p_attendeeCn = _line.attendeeCn\n                \
             if (_p_attendeeCn != null) out.param(\"CN\", _p_attendeeCn, 24)?.let { return it }\n                \
             out.param(\"ROLE\", _line.attendeeRole, 16)?.let { return it }\n                \
             out.string(_line.value, false, 40)?.let { return it }\n            }\n",
        ],
    );
}

/// A codec whose plan uses a form the Kotlin printer does not lower keeps the
/// template's encode whole: a list on one side of a gate held to a law names
/// its form.
///
/// Turns red where `kotlin_source::positional_forms` stops naming a list a
/// group holds held to a law.
#[test]
fn a_codec_of_a_form_the_printer_does_not_lower_keeps_the_template_encode() {
    let name = "codec_repeat_flag_group";
    let form = "a list on one side of a gate held to a law";
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

/// The documents `stem` imports, by stem, as its `<sce:import src=...>` names
/// them.
fn imports_of(stem: &str) -> Vec<String> {
    let text = std::fs::read_to_string(resources().join(format!("{stem}.scxml")))
        .unwrap_or_else(|e| panic!("{stem} reads: {e}"));
    text.split("<sce:import")
        .skip(1)
        .filter_map(|declaration| {
            let declaration = &declaration[..declaration.find('>')?];
            let start = declaration.find("src=\"")? + "src=\"".len();
            let src = &declaration[start..];
            src[..src.find('"')?]
                .strip_suffix(".scxml")
                .map(str::to_string)
        })
        .collect()
}

/// Whether the generator refuses the document at `path` on Kotlin for the
/// target the document states, not for a form the printer does not lower: the
/// typed refusal `reject_mcu_only_codec` answers for a codec of MCU-only
/// features on a backend that is not MCU-class
/// (`codegen/mcu-class-kind-on-non-mcu-language`). The generator's own verdict
/// is read, so the census leaves out what it leaves out and no name list
/// stands in for it; any other refusal is the caller's failure to report.
fn refused_on_kotlin_for_its_target(path: &Path, options: &ForgeCompileOptions) -> bool {
    match compile_forge_file(path, Language::Kotlin, &[], options) {
        Ok(_) => false,
        Err(refusal) => match refusal.error {
            ForgeError::Generate(boxed) => matches!(
                *boxed,
                GenerateError::CodegenMcuClassKindOnNonMcuLanguage { .. }
            ),
            _ => false,
        },
    }
}

/// Every codec of the resources the Kotlin printer prints compiles under
/// `-Werror`, with every document it imports and the forge runtime it names:
/// what the printer writes is held by the compiler, not only by the text the
/// tests above read. Skipped where no `kotlinc` is on the path.
///
/// Turns red where `kotlin_source` writes a statement kotlinc refuses or warns
/// of: a smart cast it cannot make, a parameter or a local it leaves unread, a
/// literal of the wrong type, a conversion it calls redundant.
#[test]
fn every_printed_kotlin_codec_compiles_under_werror() {
    let Some(kotlinc) = sce_build::toolchain::require_any_or_skip(
        &["kotlinc"],
        "compile the codecs the Kotlin printer prints",
    ) else {
        return;
    };
    let options = ForgeCompileOptions::default();
    // The printed codecs Kotlin generates, and every document they import,
    // transitively. A codec whose document states a target Kotlin is not, one
    // of MCU-only features, is refused before the printer reads a form
    // (`generator::reject_mcu_only_codec`): it has no Kotlin to compile.
    let mut wanted: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut queue: Vec<String> = Vec::new();
    let mut paths: Vec<PathBuf> = std::fs::read_dir(resources())
        .expect("the resources directory")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "scxml"))
        .collect();
    paths.sort();
    for path in &paths {
        if kotlin_codec_printed_file(path, &[], &options).ok() == Some(Some(true))
            && !refused_on_kotlin_for_its_target(path, &options)
        {
            let stem = path
                .file_stem()
                .expect("a document has a stem")
                .to_string_lossy()
                .into_owned();
            if wanted.insert(stem.clone()) {
                queue.push(stem);
            }
        }
    }
    while let Some(stem) = queue.pop() {
        for imported in imports_of(&stem) {
            if wanted.insert(imported.clone()) {
                queue.push(imported);
            }
        }
    }
    let dir = tempfile::tempdir().expect("a scratch directory");
    let mut sources: Vec<PathBuf> = Vec::new();
    for stem in &wanted {
        let output = compile_forge_file(
            &resources().join(format!("{stem}.scxml")),
            Language::Kotlin,
            &[],
            &options,
        )
        .unwrap_or_else(|e| panic!("{stem} generates on Kotlin: {:?}", e.error));
        for (file, text) in output.files {
            // A test-vector sidecar needs the test library, which is not on
            // this compile's path.
            if !file.ends_with(".kt") || file.ends_with("TestVectors.kt") {
                continue;
            }
            let path = dir.path().join(&file);
            std::fs::write(&path, text).unwrap_or_else(|e| panic!("write {file}: {e}"));
            sources.push(path);
        }
    }
    let runtime = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../backends/kotlin/forge-runtime/src/commonMain/kotlin/com/sce/forge/runtime");
    sources.push(runtime.join("Codec.kt"));
    sources.push(runtime.join("Utf8.kt"));
    sources.push(runtime.join("Aligned.kt"));
    sources.push(runtime.join("Cbor.kt"));
    sources.push(runtime.join("ContentLine.kt"));
    let out = common::kotlin::kotlinc(&kotlinc)
        .arg("-Werror")
        .arg("-d")
        .arg(dir.path().join("classes"))
        .args(&sources)
        .output()
        .expect("kotlinc runs");
    assert!(
        out.status.success(),
        "the printed Kotlin codecs ({}) do not compile under -Werror:\n{}",
        wanted.iter().cloned().collect::<Vec<_>>().join(", "),
        String::from_utf8_lossy(&out.stderr)
    );
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
        "codec_variant_dispatch",
        "codec_source_parent_tag",
        "codec_present_if_length_ref",
        "codec_present_if_disjunction",
        "codec_zenoh_init_body",
        "codec_repeat_unconditional_count",
        "codec_chain_handed",
        "codec_cbor_map",
        "codec_content_line_event",
        "codec_content_line_records",
        "codec_content_line_enums",
        "codec_content_line_open",
        "codec_content_line_required_lists",
    ] {
        assert!(
            printed.iter().any(|stem| stem == name),
            "{name} is printed on Kotlin"
        );
    }
}
