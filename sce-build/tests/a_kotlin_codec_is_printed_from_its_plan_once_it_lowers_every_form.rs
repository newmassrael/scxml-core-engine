// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
            "        fun of(bytes: ByteArray): CodecLengthRefPayloadBytes? = \
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

/// A codec whose plan uses a form the Kotlin printer does not lower keeps the
/// template's encode whole: a variant, a CBOR map and an element a gate
/// decides each name their form.
///
/// Turns red where `kotlin_source::positional_forms` stops naming a variant or
/// a gated element, or `unlowered_forms` a CBOR map.
#[test]
fn a_codec_of_a_form_the_printer_does_not_lower_keeps_the_template_encode() {
    for (name, form) in [
        ("codec_variant_default_marker", "a variant"),
        ("codec_cbor_map", "a CBOR map"),
        ("codec_present_if_basic", "an element a gate decides"),
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
    ] {
        assert!(
            printed.iter().any(|stem| stem == name),
            "{name} is printed on Kotlin"
        );
    }
}
