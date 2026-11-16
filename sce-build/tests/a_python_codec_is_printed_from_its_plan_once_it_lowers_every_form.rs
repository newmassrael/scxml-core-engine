// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Which codecs the Python backend prints from their encode plan
//! (docs/adr/0081 step 4, §6), and what a printed one is.
//!
//! The Python printer lowers the plan's forms one landing at a time. A codec
//! whose every form it lowers is printed: its module names a `Protocol` of the
//! elements a source presents, an encode over any source that derives every
//! other element of the frame (docs/adr/0058 §1), and a value that holds no
//! element the encode derives and holds each byte string in the class of its
//! bound. Every other codec keeps the template's encode whole, and the printer
//! names the first form it does not lower.

use std::path::{Path, PathBuf};

use sce_build::generator::Language;
use sce_build::{
    compile_forge_file, python_codec_printed_file, python_codec_unlowered_file, ForgeCompileOptions,
};

/// The directory of the resources.
fn resources() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/resources")
}

/// The Python module the document `path` generates.
fn module_of(path: &Path) -> Result<String, String> {
    let stem = path
        .file_stem()
        .expect("a document has a stem")
        .to_string_lossy()
        .into_owned();
    let output = compile_forge_file(path, Language::Python, &[], &ForgeCompileOptions::default())
        .map_err(|e| format!("{:?}", e.error))?;
    output
        .files
        .into_iter()
        .find(|(file, _)| *file == format!("{stem}.py"))
        .map(|(_, text)| text)
        .ok_or_else(|| format!("{stem} generates no `{stem}.py`"))
}

/// The Python module of the resource `name`, which generates.
fn resource_module(name: &str) -> String {
    module_of(&resources().join(format!("{name}.scxml")))
        .unwrap_or_else(|e| panic!("{name} generates on Python: {e}"))
}

/// The forms of the resource `name` the Python printer does not lower.
fn unlowered(name: &str) -> Vec<String> {
    python_codec_unlowered_file(
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
            "{name}: the module holds `{needle}`:\n{text}"
        );
    }
}

/// Whether `text` holds none of `needles`, with `text` in the failure.
fn holds_none(name: &str, text: &str, needles: &[&str]) {
    for needle in needles {
        assert!(
            !text.contains(needle),
            "{name}: the module holds no `{needle}`:\n{text}"
        );
    }
}

/// A length of a byte string is derived on encode from the bytes the source
/// presents, and is no field of the value; the byte string is presented in the
/// class of its bound, which the value's constructor holds it to.
///
/// Turns red where `python_source::encode_source` reads the length off the
/// source rather than measuring the bytes, where the template's dataclass loop
/// stops skipping a derived field, or where `render` names no bound class.
#[test]
fn a_length_is_derived_from_the_bytes_it_measures_and_is_no_field_of_the_value() {
    assert!(unlowered("codec_length_ref").is_empty());
    let text = resource_module("codec_length_ref");
    holds(
        "codec_length_ref",
        &text,
        &[
            "from sce_forge_runtime.bounded import BoundedBytes\n",
            "from typing import Optional, Protocol\n",
            "class CodecLengthRefPayloadBytes(BoundedBytes):\n",
            "    LEAST = 0\n    MOST = 32\n",
            "class CodecLengthRefSource(Protocol):\n",
            "    @property\n    def msg_id(self) -> int: ...\n",
            "    @property\n    def payload(self) -> CodecLengthRefPayloadBytes: ...\n",
            "    payload: CodecLengthRefPayloadBytes = CodecLengthRefPayloadBytes()\n",
            "    def __setattr__(self, name: str, value: object) -> None:\n",
            "        if name == \"payload\":\n            value = CodecLengthRefPayloadBytes(value)\n",
            "        object.__setattr__(self, name, value)\n",
            "        encode_source(self, w)\n",
            "\ndef encode_source(src: CodecLengthRefSource, w: SceSink) -> None:\n",
            "    _d_len = len(src.payload)\n",
            "    w.write_u8(src.msg_id & 0xFF)\n",
            "    w.write_u8(_d_len & 0xFF)\n",
            "    w.write_bytes(src.payload)\n",
        ],
    );
    // The value holds no length, is made without one, and the encode reads
    // none off the value; it refuses no bound, which the class holds.
    holds_none(
        "codec_length_ref",
        &text,
        &[
            "\n    len: ",
            "            len=",
            "src.len",
            "self.len",
            "if len(self.payload) > 32:",
        ],
    );
}

/// A length its `sce:length-arith` takes off is derived less that, and the
/// class of the bytes holds at least what it takes off, so a value made by
/// default is within its bound.
///
/// Turns red where `python_source::encode_source` drops the `Offset::Less`
/// arithmetic, or where `render` defaults a byte string of a positive least to
/// no bytes.
#[test]
fn a_length_less_its_arithmetic_is_derived_less_it() {
    assert!(unlowered("codec_scout_zid_body").is_empty());
    let text = resource_module("codec_scout_zid_body");
    holds(
        "codec_scout_zid_body",
        &text,
        &[
            "class CodecScoutZidBodyZidBytes(BoundedBytes):\n",
            "    LEAST = 1\n    MOST = 16\n",
            "    zid: CodecScoutZidBodyZidBytes = CodecScoutZidBodyZidBytes(b\"\\x00\" * 1)\n",
            "    _d_zid_len_m1 = len(src.zid) - 1\n",
            "    w.write_u8(_d_zid_len_m1 & 0xFF)\n",
            "    w.write_bytes(src.zid)\n",
        ],
    );
    holds_none(
        "codec_scout_zid_body",
        &text,
        &["\n    zid_len_m1: ", "src.zid_len_m1"],
    );
}

/// A codec of integers alone is printed with no class of a bound, its value's
/// fields unchanged.
///
/// Turns red where `python_source::positional_forms` refuses an integer
/// element, or where the template imports `BoundedBytes` for a codec that
/// presents no byte string.
#[test]
fn a_codec_of_integers_is_printed_with_no_class_of_a_bound() {
    assert!(unlowered("codec_simple_frame").is_empty());
    let text = resource_module("codec_simple_frame");
    holds(
        "codec_simple_frame",
        &text,
        &[
            "class CodecSimpleFrameSource(Protocol):\n",
            "\ndef encode_source(src: CodecSimpleFrameSource, w: SceSink) -> None:\n",
            "        encode_source(self, w)\n",
        ],
    );
    holds_none(
        "codec_simple_frame",
        &text,
        &["BoundedBytes", "__setattr__", "_d_"],
    );
}

/// An element a gate decides is `None` where it is absent, written only where
/// it is there, and the flag that gates it is derived from it and folded into
/// its carrier in place of the value's bit.
///
/// Turns red where `python_source::encode_source` reads the presence flag off
/// the value rather than folding it from the gated element, or writes the
/// gated element whether or not it is there.
#[test]
fn a_gated_element_derives_its_flag_and_is_written_where_it_is_there() {
    assert!(unlowered("codec_present_if_basic").is_empty());
    let text = resource_module("codec_present_if_basic");
    holds(
        "codec_present_if_basic",
        &text,
        &[
            "    @property\n    def seq(self) -> Optional[int]: ...\n",
            "    _c_flags = src.flags\n",
            "    _c_flags = (_c_flags & ~0x1) | (0x1 if src.seq is not None else 0)\n",
            "    w.write_u8(_c_flags & 0xFF)\n",
            "    if src.seq is not None:\n        w.write_u8((src.seq >> 8) & 0xFF)\n        \
             w.write_u8(src.seq & 0xFF)\n",
        ],
    );
}

/// A codec whose plan uses a form the Python printer does not lower keeps the
/// template's encode whole: a body and a CBOR map each name their form, and a
/// length the value states where its gated content is absent names one.
///
/// Turns red where `python_source::positional_forms` stops naming an embedded
/// body or a stated length, or `unlowered_forms` a CBOR map.
#[test]
fn a_codec_of_a_form_the_printer_does_not_lower_keeps_the_template_encode() {
    assert!(
        !unlowered("codec_present_if_length_ref").is_empty(),
        "a length the value states beside gated content is not printed yet"
    );
    for (name, form) in [
        ("codec_embed_basic", "an embedded body"),
        ("codec_cbor_map", "a CBOR map"),
    ] {
        let forms = unlowered(name);
        assert!(
            forms.iter().any(|f| f.contains(form)),
            "{name}: the printer names `{form}` among {forms:?}"
        );
        let text = resource_module(name);
        holds_none(
            name,
            &text,
            &["def encode_source(", "(Protocol):", "BoundedBytes"],
        );
    }
}

/// Every codec of the resources is printed exactly where the printer lowers
/// each of its forms, and each printed one generates a module with its encode
/// over a source. The printed codecs are shown with `--nocapture`.
///
/// Turns red where `python_codec_printed` and `python_codec_unlowered` come
/// apart, where `render_codec` prints a codec the printer does not lower or
/// leaves one it lowers on the template's encode, or where the printer panics
/// or the template fails on a codec it lowers.
#[test]
fn the_python_backend_prints_each_codec_of_the_resources_whose_forms_it_lowers() {
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
            python_codec_unlowered_file(path, &[], &ForgeCompileOptions::default())
        else {
            continue;
        };
        assert_eq!(
            python_codec_printed_file(path, &[], &ForgeCompileOptions::default()).ok(),
            Some(Some(forms.is_empty())),
            "{stem}: printed exactly where no form is unlowered"
        );
        match module_of(path) {
            Ok(text) => assert_eq!(
                text.contains("\ndef encode_source(src: "),
                forms.is_empty(),
                "{stem}: the module prints its encode over a source exactly where the \
                 printer lowers every form ({forms:?})"
            ),
            // A codec of MCU-only features is generated on no host backend,
            // Python among them, whatever its forms; one the printer prints
            // generates otherwise.
            Err(refusal) => assert!(
                !forms.is_empty() || refusal.contains("CodegenMcuClassKindOnNonMcuLanguage"),
                "{stem}: a codec the printer prints generates on Python: {refusal}"
            ),
        }
        if forms.is_empty() {
            printed.push(stem);
        }
    }
    eprintln!("{} codec(s) printed on Python:", printed.len());
    for stem in &printed {
        eprintln!("  {stem}");
    }
    for name in [
        "codec_length_ref",
        "codec_scout_zid_body",
        "codec_simple_frame",
    ] {
        assert!(
            printed.iter().any(|stem| stem == name),
            "{name} is printed on Python"
        );
    }
}
