// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Which codecs the Go backend prints from their encode plan (docs/adr/0081
//! step 4, §6), and what a printed one is.
//!
//! The Go printer lowers the plan's forms one landing at a time. A codec whose
//! every form it lowers is printed: its file names an interface of the
//! elements a source presents, a value that holds them in unexported fields
//! and presents each through a method, a constructor that makes the value, and
//! an encode over any source. A parent, a procedure and a validator read such
//! a codec through its methods and make it by its constructor. Every other
//! codec keeps the template's encode whole, and the printer names the first
//! form it does not lower.

use std::path::{Path, PathBuf};

use sce_build::generator::Language;
use sce_build::{
    compile_forge_with_imports, go_codec_printed_file, go_codec_unlowered_file, DocumentLabel,
    ForgeCompileOptions,
};

/// The directory of the resources.
fn resources() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/resources")
}

/// The Go file the resource `name` generates as its own, or why it does not.
fn go_of(name: &str) -> Result<String, String> {
    let text = std::fs::read_to_string(resources().join(format!("{name}.scxml")))
        .map_err(|e| format!("read {name}.scxml: {e}"))?;
    let options = ForgeCompileOptions {
        go_module_prefix: Some("example.com/sce-forge".to_string()),
        ..ForgeCompileOptions::default()
    };
    let output = compile_forge_with_imports(
        &text,
        DocumentLabel::symmetric(name),
        Language::Go,
        &resources(),
        &options,
    )
    .map_err(|e| format!("{:?}", e.error))?;
    output
        .files
        .into_iter()
        .find(|(file, _)| {
            file.ends_with(".go") && Path::new(file).file_stem().is_some_and(|s| s == name)
        })
        .map(|(_, source)| source)
        .ok_or_else(|| format!("{name} generates no Go file of its own"))
}

/// The Go file of the resource `name`, which generates.
fn resource_go(name: &str) -> String {
    go_of(name).unwrap_or_else(|e| panic!("{name} generates on Go: {e}"))
}

/// The forms of the resource `name` the Go printer does not lower.
fn unlowered(name: &str) -> Vec<String> {
    go_codec_unlowered_file(
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

/// A codec of integers is read through the interface of its source, holds
/// each element in an unexported field behind its method, is made by its
/// constructor, and is encoded over any source, each byte read off the source.
///
/// Turns red where `go_source::render` names a method other than the field's
/// Go name, where `go_source::member` stops lowering the first letter, where
/// the template declares the struct's fields by `field.id` instead of
/// `field.member`, or where its `Encode` keeps the template's writes over
/// `s.` instead of calling `EncodeSource`.
#[test]
fn a_codec_of_integers_is_read_through_methods_and_made_by_its_constructor() {
    assert!(unlowered("codec_simple_frame").is_empty());
    let text = resource_go("codec_simple_frame");
    holds(
        "codec_simple_frame",
        &text,
        &[
            "type CodecSimpleFrameSource interface {\n\tMsgId() uint8\n\tLength() uint8\n\tPayload() uint16\n}",
            "type CodecSimpleFrame struct {\n\tmsgId uint8\n\tlength uint8\n\tpayload uint16\n}",
            "func MakeCodecSimpleFrame(msgId uint8, length uint8, payload uint16) CodecSimpleFrame {\n\treturn CodecSimpleFrame{msgId: msgId, length: length, payload: payload}\n}",
            "func (s *CodecSimpleFrame) MsgId() uint8 {\n\treturn s.msgId\n}",
            "func (s *CodecSimpleFrame) Payload() uint16 {\n\treturn s.payload\n}",
            "func EncodeSource(src CodecSimpleFrameSource, w codec.SceSink) error {\n",
            "\tif err := w.WriteBytes([]byte{byte(src.MsgId())}); err != nil {\n",
            "\tif err := w.WriteBytes([]byte{byte(src.Payload() >> 8 & 0xFF), byte(src.Payload() & 0xFF)}); err != nil {\n",
            "\treturn EncodeSource(s, w)\n",
            "\t\tmsgId: MsgId,\n",
        ],
    );
    // No element is a field of the value's own name, and the encode reads
    // nothing off the value but through the source.
    holds_none(
        "codec_simple_frame",
        &text,
        &["\tMsgId uint8\n", "s.MsgId", "byte(s."],
    );
}

/// The flag accessors of a printed codec read and write the carrier where the
/// value holds it, its unexported field; the source presents the carrier
/// whole.
///
/// Turns red where the template's flag accessors read `s.{{ field.id }}`
/// rather than `s.{{ field.member }}`.
#[test]
fn a_flag_of_a_printed_codec_reads_the_carrier_its_value_holds() {
    assert!(unlowered("codec_flags_basic").is_empty());
    let text = resource_go("codec_flags_basic");
    holds(
        "codec_flags_basic",
        &text,
        &[
            "type CodecFlagsBasicSource interface {\n\tHeader() uint8\n}",
            "func (s *CodecFlagsBasic) Reliable() bool {\n\treturn (s.header & 0x80) != 0\n}",
            "\t\ts.header |= 0x80\n",
            "func (s *CodecFlagsBasic) Header() uint8 {\n\treturn s.header\n}",
        ],
    );
    holds_none("codec_flags_basic", &text, &["s.Header"]);
}

/// A byte string is presented as the runtime's `codec.Bytes` of a bound type
/// the codec names, made only within it, and the length that counts it is no
/// element: the encode derives it from the bytes, and the decode reads it to
/// cut them and makes the bounded value before the value's literal.
///
/// Turns red where `go_source::render` stops naming the bound type or
/// presenting the element as its `codec.Bytes`, where `encode_source` reads
/// the length off the source instead of deriving it, where `derived_field`
/// stops naming the length (the struct and the constructor would hold it), or
/// where the template's decode literal holds the slice it read rather than
/// the bounded value (`field.made`).
#[test]
fn a_byte_string_is_held_to_its_bound_and_its_length_is_derived() {
    assert!(unlowered("codec_length_ref").is_empty());
    let text = resource_go("codec_length_ref");
    holds(
        "codec_length_ref",
        &text,
        &[
            "type CodecLengthRefPayloadBound struct{}\n",
            "func (CodecLengthRefPayloadBound) Least() int { return 0 }\n",
            "func (CodecLengthRefPayloadBound) Most() int { return 32 }\n",
            "type CodecLengthRefSource interface {\n\tMsgId() uint8\n\tPayload() codec.Bytes[CodecLengthRefPayloadBound]\n}",
            "type CodecLengthRef struct {\n\tmsgId uint8\n\tpayload codec.Bytes[CodecLengthRefPayloadBound]\n}",
            "func MakeCodecLengthRef(msgId uint8, payload codec.Bytes[CodecLengthRefPayloadBound]) CodecLengthRef {\n",
            "\t_dLen := uint8(src.Payload().Len())\n",
            "\tif err := w.WriteBytes([]byte{byte(_dLen)}); err != nil {\n",
            "\tif err := w.WriteBytes(src.Payload().Bytes()); err != nil {\n",
            "\t_bPayload, err := codec.MakeBytes[CodecLengthRefPayloadBound](Payload)\n",
            "\t\tpayload: _bPayload,\n",
        ],
    );
    // The length is no field, no method and no parameter of the value.
    holds_none(
        "codec_length_ref",
        &text,
        &[
            "\tlen uint8\n",
            "Len() uint8",
            "len uint8,",
            "\t\tlen: Len,",
            "src.Len()",
        ],
    );
}

/// A text is presented as the runtime's `codec.Text` of its bound, which
/// holds well-formed UTF-8 alone; the length is derived from its UTF-8 bytes,
/// and an algorithm over a collection of it reads the string the text holds.
///
/// Turns red where `go_source::render` presents a text as `codec.Bytes` or as
/// a bare `string`, where the encode writes the text other than as its bytes,
/// or where `go_source::element_read` stops reading a text as its string.
#[test]
fn a_text_is_held_to_its_bound_and_read_as_its_string() {
    assert!(unlowered("keyexpr_entry").is_empty());
    let text = resource_go("keyexpr_entry");
    holds(
        "keyexpr_entry",
        &text,
        &[
            "func (KeyexprEntryPatternBound) Most() int { return 32 }\n",
            "\tPattern() codec.Text[KeyexprEntryPatternBound]\n",
            "\t_dPatternLen := uint8(src.Pattern().Len())\n",
            "\tif err := w.WriteBytes([]byte(src.Pattern().String())); err != nil {\n",
            "\t_bPattern, err := codec.MakeText[KeyexprEntryPatternBound](string(Pattern))\n",
        ],
    );
    holds_none(
        "keyexpr_entry",
        &text,
        &["PatternLen() uint8", "\tpatternLen uint8\n"],
    );
}

/// A chain reads the identifier of an entry the Go backend prints through the
/// entry's method, as the entry holds it in an unexported field.
///
/// Turns red where `chain_identifier_masks` asks only `cpp_printed` of the
/// entry's import, or where `lib.rs` stops setting `go_printed` on a Go
/// compile.
#[test]
fn a_chain_reads_the_identifier_of_a_printed_entry_through_its_method() {
    assert!(unlowered("codec_chain_has_tagged_entry").is_empty());
    assert!(
        !unlowered("codec_chain_has_tagged").is_empty(),
        "the chain itself is not printed yet"
    );
    let text = resource_go("codec_chain_has_tagged");
    holds(
        "codec_chain_has_tagged",
        &text,
        &["uint64(_e.EntryType()) == 7", "uint64(_e.EntryType()) == 9"],
    );
    holds_none("codec_chain_has_tagged", &text, &["_e.EntryType)"]);
}

/// A procedure that assigns an element of a codec the Go backend prints makes
/// the value again by its constructor, each other element read through its
/// method, and a validator reads an element through its method.
///
/// Turns red where `rebuilt_element_assign` answers only for C++, where the Go
/// procedure is handed no imports for it, or where
/// `stateful_import_field_renames` reads a printed Go codec's element as a
/// field.
#[test]
fn a_procedure_makes_a_printed_codec_again_and_a_validator_reads_it_by_method() {
    let procedure = resource_go("crossfile_procedure_codec_mutate");
    holds(
        "crossfile_procedure_codec_mutate",
        &procedure,
        &["p.Frame = codec_simple_frame.MakeCodecSimpleFrame(p.msgId, p.Frame.Length(), p.Frame.Payload())"],
    );
    holds_none(
        "crossfile_procedure_codec_mutate",
        &procedure,
        &["p.Frame.MsgId ="],
    );
    let validator = resource_go("crossfile_validator_codec");
    holds(
        "crossfile_validator_codec",
        &validator,
        &["sceSelf.Frame.MsgId() == msgId"],
    );
}

/// A codec of a form the Go printer does not lower keeps the template's
/// encode whole: its fields stay exported, and it has no source, constructor
/// or encode over a source.
///
/// Turns red where `go_source::positional_forms` stops naming a gated
/// element, a variant or a CBOR map, or where `render_codec` prints a codec
/// it names a form of.
#[test]
fn a_codec_of_a_form_the_printer_does_not_lower_keeps_the_template_encode() {
    for (name, form) in [
        ("codec_present_if_basic", "an element a gate decides"),
        ("codec_variant_default_marker", "a variant"),
        ("codec_cbor_map", "a CBOR map"),
    ] {
        let forms = unlowered(name);
        assert!(
            forms.iter().any(|f| f.contains(form)),
            "{name}: the printer names `{form}` among {forms:?}"
        );
        let text = resource_go(name);
        holds_none(
            name,
            &text,
            &["func EncodeSource(", "Source interface {", "func Make"],
        );
    }
}

/// Every codec of the resources is printed exactly where the printer lowers
/// each of its forms, and each printed one generates a file with its encode
/// over a source. The printed codecs are shown with `--nocapture`.
///
/// Turns red where `go_codec_printed` and `go_codec_unlowered` come apart,
/// where `render_codec` prints a codec the printer does not lower or leaves
/// one it lowers on the template's encode, or where the printer panics or the
/// template fails on a codec it lowers.
#[test]
fn the_go_backend_prints_each_codec_of_the_resources_whose_forms_it_lowers() {
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
        let Ok(Some(forms)) = go_codec_unlowered_file(path, &[], &ForgeCompileOptions::default())
        else {
            continue;
        };
        assert_eq!(
            go_codec_printed_file(path, &[], &ForgeCompileOptions::default()).ok(),
            Some(Some(forms.is_empty())),
            "{stem}: printed exactly where no form is unlowered"
        );
        match go_of(&stem) {
            Ok(text) => assert_eq!(
                text.contains("\nfunc EncodeSource(src "),
                forms.is_empty(),
                "{stem}: the file prints its encode over a source exactly where the \
                 printer lowers every form ({forms:?})"
            ),
            // A codec of MCU-only features is generated on no host backend, Go
            // among them, whatever its forms; one the printer prints generates
            // otherwise.
            Err(refusal) => assert!(
                !forms.is_empty() || refusal.contains("CodegenMcuClassKindOnNonMcuLanguage"),
                "{stem}: a codec the printer prints generates on Go: {refusal}"
            ),
        }
        if forms.is_empty() {
            printed.push(stem);
        }
    }
    eprintln!("{} codec(s) printed on Go:", printed.len());
    for stem in &printed {
        eprintln!("  {stem}");
    }
    for name in [
        "codec_simple_frame",
        "codec_flags_basic",
        "codec_chain_has_tagged_entry",
        "codec_length_ref",
        "codec_tail",
        "keyexpr_entry",
        "broadcast_log_conformance_record",
        "queue_conformance_event",
        "broadcast_log_conformance_event",
    ] {
        assert!(
            printed.iter().any(|stem| stem == name),
            "{name} is printed on Go"
        );
    }
}
