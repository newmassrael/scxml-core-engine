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
    go_of_document(name, &text)
}

/// The Go file the document `text`, named `name` and importing from the
/// resources, generates as its own, or why it does not.
fn go_of_document(name: &str, text: &str) -> Result<String, String> {
    let options = ForgeCompileOptions {
        go_module_prefix: Some("example.com/sce-forge".to_string()),
        ..ForgeCompileOptions::default()
    };
    let output = compile_forge_with_imports(
        text,
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
    // The length is no field, no element the source presents and no
    // parameter of the value: the value answers it by the method that
    // derives it.
    holds(
        "codec_length_ref",
        &text,
        &["func (s *CodecLengthRef) WireLen() uint8 {\n\treturn uint8(s.payload.Len())\n}\n"],
    );
    holds_none(
        "codec_length_ref",
        &text,
        &[
            "\tlen uint8\n",
            "\tLen() uint8",
            "CodecLengthRef) Len()",
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
        &["\tPatternLen() uint8", "\tpatternLen uint8\n"],
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

/// An element a gate decides is presented as the optional type of what it
/// holds, and the flag its presence says is derived and folded into its
/// carrier in place of the bit the value presents; a gate the encode cannot
/// derive the presence from holds the element it gates, tested on the carrier
/// as the encode writes it, and refuses a value it disagrees with before the
/// first byte.
///
/// Turns red where `go_source::render` presents a gated element as what it
/// holds rather than its optional type, where the encode reads the carrier's
/// bit off the source instead of folding the presence in, where the gate
/// check reads `src.Flags()` instead of the folded carrier, or where
/// `refusal` stops returning the runtime's sentinel.
#[test]
fn a_gated_element_derives_its_flag_and_a_checked_gate_refuses_a_disagreement() {
    assert!(unlowered("codec_present_if_checked").is_empty());
    let text = resource_go("codec_present_if_checked");
    holds(
        "codec_present_if_checked",
        &text,
        &[
            "\tSeq() optional.Of[uint16]\n",
            "\tTrailer() optional.Of[uint8]\n",
            "func MakeCodecPresentIfChecked(flags uint8, seq optional.Of[uint16], trailer optional.Of[uint8]) CodecPresentIfChecked {\n",
            "\t_cFlags := src.Flags()\n\t_cFlags &^= 0x1\n\tif src.Seq().IsPresent() {\n\t\t_cFlags |= 0x1\n\t}\n",
            "\tif (((_cFlags >> 0) & 1) != 0 || ((_cFlags >> 1) & 1) != 0) != src.Trailer().IsPresent() {\n\t\treturn codec.ErrPresentIfMismatch\n\t}\n",
            "\tif src.Trailer().IsPresent() {\n",
            "func (s *CodecPresentIfChecked) EncodeToBytes() ([]byte, error) {\n",
        ],
    );
    holds_none(
        "codec_present_if_checked",
        &text,
        &["s.Flags", "src.Flags() >>"],
    );
}

/// A length two byte strings share is derived from the first, the second
/// held to say the same, and refused where it does not fit its field.
///
/// Turns red where `derive_number` stops comparing every measure always there
/// with the first, or stops checking a length its field may not hold.
#[test]
fn a_shared_length_is_derived_once_and_two_that_disagree_are_refused() {
    assert!(unlowered("codec_length_shared").is_empty());
    let text = resource_go("codec_length_shared");
    holds(
        "codec_length_shared",
        &text,
        &[
            "\tif src.Beta().Len() != src.Alpha().Len() {\n\t\treturn codec.ErrInconsistentSource\n\t}\n",
            "\tif src.Alpha().Len() > 0xFF {\n\t\treturn codec.ErrLengthOutOfRange\n\t}\n",
            "\t_dSpanLen := uint8(src.Alpha().Len())\n",
            "func (s *CodecLengthShared) WireSpanLen() uint8 {\n\treturn uint8(s.alpha.Len())\n}\n",
        ],
    );
    holds_none(
        "codec_length_shared",
        &text,
        &["\tSpanLen() uint8", "\tspanLen uint8\n"],
    );
}

/// A length of content a gate decides is answered as the frame carries it:
/// where the length is gated beside its content, the optional type of the
/// number, present exactly where the content is; where the value states it
/// while its content is absent, the first content there, and otherwise what
/// the value states.
///
/// Turns red where `wire_methods` reads a gated content without testing its
/// presence, answers a length gated beside its content as a bare number, or
/// stops falling back to what the value states.
#[test]
fn a_length_of_gated_content_is_answered_as_the_frame_carries_it() {
    assert!(unlowered("codec_present_if_string").is_empty());
    let text = resource_go("codec_present_if_string");
    holds(
        "codec_present_if_string",
        &text,
        &[
            "func (s *CodecPresentIfString) WireTextLen() optional.Of[uint8] {\n\
           \tif _v, ok := s.text.Get(); ok {\n\
           \t\treturn optional.Some(uint8(_v.Len()))\n\
           \t}\n\
           \treturn optional.None[uint8]()\n\
           }\n",
        ],
    );
    assert!(unlowered("codec_length_stated").is_empty());
    let text = resource_go("codec_length_stated");
    holds(
        "codec_length_stated",
        &text,
        &["func (s *CodecLengthStated) WireSpanLen() uint8 {\n\
           \tif _v, ok := s.alpha.Get(); ok {\n\
           \t\treturn uint8(_v.Len())\n\
           \t}\n\
           \tif _v, ok := s.beta.Get(); ok {\n\
           \t\treturn uint8(_v.Len())\n\
           \t}\n\
           \tif _s, ok := s.spanLen.Get(); ok {\n\
           \t\treturn _s\n\
           \t}\n\
           \treturn 0\n\
           }\n"],
    );
}

/// A procedure over `codec_length_ref` that assigns `location` from `expr`,
/// its input `blob` a byte string of at most 32 bytes and `n` an integer.
fn length_ref_procedure(location: &str, expr: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="procedure" initial="start" version="1.0">
  <sce:import src="codec_length_ref.scxml" kind="codec" as="frame"/>
  <datamodel>
    <data id="blob" sce:type="bytes" sce:direction="in" sce:max-size="32"/>
    <data id="n" sce:type="uint8" sce:direction="in"/>
  </datamodel>
  <state id="start">
    <transition target="done">
      <assign location="{location}" expr="{expr}"/>
    </transition>
  </state>
  <final id="done">
    <donedata><param name="result" expr="'success'"/></donedata>
  </final>
</scxml>
"#
    )
}

/// A procedure that assigns a byte string of a printed codec makes its
/// bounded value by the runtime's constructor first, raising
/// `error.execution` where the constructor refuses it, and makes the value
/// again from the elements it presents alone: the length the encode derives
/// is no argument of `Make{Codec}`.
///
/// Turns red where `go_source::GoPrinted::rebuilt` hands `Make{Codec}` every
/// field the codec declares (the length among them), where it hands the
/// assigned slice without `codec.MakeBytes`, where the template drops the
/// `a.made` branch, or where `render_procedure_go` stops setting
/// `uses_forge_codec`, which leaves the `codec` package unimported.
#[test]
fn a_procedure_makes_a_byte_string_of_a_printed_codec_within_its_bound() {
    assert!(unlowered("codec_length_ref").is_empty());
    let name = "go_procedure_assigns_printed_bytes";
    let procedure = go_of_document(name, &length_ref_procedure("frame.payload", "blob"))
        .unwrap_or_else(|e| panic!("{name} generates on Go: {e}"));
    holds(
        name,
        &procedure,
        &[
            "\t\"github.com/newmassrael/sce-forge-runtime/codec\"\n",
            "sceValue := p.blob\n",
            "sceMade, err := codec.MakeBytes[codec_length_ref.CodecLengthRefPayloadBound](sceValue)\n",
            "\t\t\t\tif err != nil {\n\t\t\t\t\treturn eventErrorExecution, true\n\t\t\t\t}\n",
            "p.Frame = codec_length_ref.MakeCodecLengthRef(p.Frame.MsgId(), sceMade)\n",
        ],
    );
    holds_none(
        name,
        &procedure,
        &["p.Frame.Len()", "p.Frame.Payload =", "p.Frame.Len ="],
    );
}

/// A procedure that assigns an integer of a printed codec whose value holds
/// no byte string it assigns hands the integer as it is and imports no
/// `codec` package.
///
/// Turns red where `go_source::GoPrinted::rebuilt` makes a bounded value of
/// an element that is no byte string or text, or where `uses_forge_codec` is
/// set without a rebuild that makes one.
#[test]
fn a_procedure_hands_an_integer_of_a_printed_codec_as_it_is() {
    let name = "go_procedure_assigns_printed_integer";
    let procedure = go_of_document(name, &length_ref_procedure("frame.msgId", "n"))
        .unwrap_or_else(|e| panic!("{name} generates on Go: {e}"));
    holds(
        name,
        &procedure,
        &["p.Frame = codec_length_ref.MakeCodecLengthRef(p.n, p.Frame.Payload())\n"],
    );
    holds_none(
        name,
        &procedure,
        &[
            "sceMade",
            "sceValue",
            "\"github.com/newmassrael/sce-forge-runtime/codec\"",
        ],
    );
}

/// An assign to a length the encode of a printed codec derives is refused:
/// the value holds no such element, as the encode derives it from the bytes
/// it measures (docs/adr/0081 §3).
///
/// Turns red where `rebuilt_element_assign` answers `None` for an element
/// `GoPrinted::rebuilt` does not find, which leaves the assign as a write of
/// a field the value does not have.
#[test]
fn a_procedure_cannot_assign_a_length_a_printed_codec_derives() {
    let name = "go_procedure_assigns_derived_length";
    let refusal = go_of_document(name, &length_ref_procedure("frame.len", "n"))
        .expect_err("an assign to a derived length is refused");
    assert!(
        refusal.contains("InvalidLvalue") && refusal.contains("derives `len`"),
        "the refusal names the derived element: {refusal}"
    );
}

/// A validator reads a length the encode of a printed codec derives by the
/// method of the value that derives it as the encode does
/// (`go_source::wire_method`), as the value holds no field for it.
///
/// Turns red where `GoPrinted::read` reads a derived element by a method of
/// the field's own name, which the value does not have, or where
/// `stateful_import_field_renames` stops reading a printed codec's element
/// through `GoPrinted::read`.
#[test]
fn a_validator_reads_a_derived_length_of_a_printed_codec_from_what_it_measures() {
    let name = "go_validator_reads_printed_length";
    let validator = go_of_document(
        name,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="validator" version="1.0">
  <sce:import src="codec_length_ref.scxml" kind="codec" as="frame"/>
  <datamodel>
    <data id="n" sce:type="uint8" sce:direction="in"
          sce:range-min="0" sce:range-max="32"/>
    <data id="valid" sce:type="bool" sce:direction="out"
          sce:plausibility="frame.len === n"/>
  </datamodel>
</scxml>
"#,
    )
    .unwrap_or_else(|e| panic!("{name} generates on Go: {e}"));
    holds(name, &validator, &["sceSelf.Frame.WireLen()"]);
    holds_none(name, &validator, &["sceSelf.Frame.Len"]);
}

/// A codec of a form the Go printer does not lower keeps the template's
/// encode whole: its fields stay exported, and it has no source, constructor
/// or encode over a source.
///
/// Turns red where `go_source::positional_forms` stops naming a chain, a
/// variant or a CBOR map, or where `render_codec` prints a codec it names a
/// form of.
#[test]
fn a_codec_of_a_form_the_printer_does_not_lower_keeps_the_template_encode() {
    for (name, form) in [
        ("codec_chain_has_marker", "a list of"),
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
        "codec_present_if_basic",
        "codec_present_if_checked",
        "codec_present_if_string",
        "codec_present_if_tail",
        "codec_length_shared",
    ] {
        assert!(
            printed.iter().any(|stem| stem == name),
            "{name} is printed on Go"
        );
    }
}
