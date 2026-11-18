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

/// The options every Go generation and reading here runs with: a module
/// prefix, without which a codec that imports another is no Go document.
fn options() -> ForgeCompileOptions {
    ForgeCompileOptions {
        go_module_prefix: Some("example.com/sce-forge".to_string()),
        ..ForgeCompileOptions::default()
    }
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
    let output = compile_forge_with_imports(
        text,
        DocumentLabel::symmetric(name),
        Language::Go,
        &resources(),
        &options(),
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
    go_codec_unlowered_file(&resources().join(format!("{name}.scxml")), &[], &options())
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

/// A decode of a codec whose byte string ends where its length says advances
/// the cursor past those bytes and no further, so a codec holding it as a body
/// or a chain's entry decodes on after it (`codec_origin_envelope`,
/// `codec_tlv_chain_basic`); a codec with a tail field consumes the frame
/// whole, the tail running to its end.
///
/// Turns red where the positional decode of `codec.go.jinja2` advances by the
/// cursor's whole remainder without a tail field, or by less than it with one.
#[test]
fn a_decode_consumes_its_own_bytes_and_a_tail_the_rest_of_the_frame() {
    let leaf = resource_go("codec_origin_leaf");
    holds(
        "codec_origin_leaf",
        &leaf,
        &["\tconsumed := 1\n\
           \tconsumed = max(consumed, 1+int(Len))\n\
           \tif err := cursor.Advance(consumed); err != nil {\n"],
    );
    holds_none("codec_origin_leaf", &leaf, &["cursor.Advance(frameLen)"]);
    let tail = resource_go("codec_tail");
    holds(
        "codec_tail",
        &tail,
        &["\tif err := cursor.Advance(frameLen); err != nil {\n"],
    );
    holds_none("codec_tail", &tail, &["consumed :="]);
}

/// The end of a length-ref field is spelled as gofmt leaves it in the guard
/// and in the argument of `max` it stands in, whatever form it takes: a length
/// adjusted by its arithmetic, and a length read from the bits of a carrier,
/// with no blank around an operator a level below the statement.
///
/// Turns red where the Go arm of `length_ref_end` in `generator.rs` puts
/// blanks around the arithmetic or the shift and mask, which gofmt removes, or
/// where `codec.go.jinja2` assigns the end at the statement's own level, where
/// gofmt adds them.
#[test]
fn the_end_of_a_length_is_spelled_as_gofmt_leaves_it() {
    let arith = resource_go("codec_scout_zid_body");
    holds(
        "codec_scout_zid_body",
        &arith,
        &[
            "\tif frameLen < 1+int(ZidLenM1)+1 {\n",
            "\tconsumed = max(consumed, 1+int(ZidLenM1)+1)\n",
        ],
    );
    let dotted = resource_go("codec_length_ref_dotted_basic");
    holds(
        "codec_length_ref_dotted_basic",
        &dotted,
        &[
            "\tif frameLen < 1+int((Carrier>>4)&0xF) {\n",
            "\tconsumed = max(consumed, 1+int((Carrier>>4)&0xF))\n",
        ],
    );
    holds_none(
        "codec_length_ref_dotted_basic",
        &dotted,
        &["int((Carrier >> 4) & 0xF)"],
    );
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
    // The accessors' reads and writes of the carrier, spelled whole: a bare
    // `s.Header` is also part of `overrides.HeaderReliable`, which the encode
    // over the source reads.
    holds_none(
        "codec_flags_basic",
        &text,
        &["(s.Header &", "s.Header |=", "s.Header &^="],
    );
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
        &["func (s *CodecLengthRef) WireLen() uint8 {\n\treturn uint8(s.Payload().Len())\n}\n"],
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
/// Turns red where `chain_identifier_masks` stops asking the entry's import
/// what a reader knows of it, or where `lib.rs` stops setting
/// `ImportContext::printed` on a Go compile.
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
        // A bare `s.Flags` is also part of `overrides.FlagsWantsTrailer`.
        &["(s.Flags &", "s.Flags |=", "s.Flags &^=", "src.Flags() >>"],
    );
}

/// A length two byte strings share is derived from the first, the second
/// held to say the same. Neither can pass what the field holds: the width of
/// the length bounds each byte string's type (docs/adr/0077 §1), so the
/// encode has no length of its field to refuse.
///
/// Turns red where `derive_number` stops comparing every measure always there
/// with the first, or where a byte string's bound stops being the most its
/// length field holds (a check the bound makes dead is then printed again).
#[test]
fn a_shared_length_is_derived_once_and_two_that_disagree_are_refused() {
    assert!(unlowered("codec_length_shared").is_empty());
    let text = resource_go("codec_length_shared");
    holds(
        "codec_length_shared",
        &text,
        &[
            "\tif src.Beta().Len() != src.Alpha().Len() {\n\t\treturn codec.ErrInconsistentSource\n\t}\n",
            "func (CodecLengthSharedAlphaBound) Most() int { return 255 }\n",
            "func (CodecLengthSharedBetaBound) Most() int { return 255 }\n",
            "\t_dSpanLen := uint8(src.Alpha().Len())\n",
            "func (s *CodecLengthShared) WireSpanLen() uint8 {\n\treturn uint8(s.Alpha().Len())\n}\n",
        ],
    );
    holds_none(
        "codec_length_shared",
        &text,
        &[
            "\tSpanLen() uint8",
            "\tspanLen uint8\n",
            "\tif src.Alpha().Len() > 0xFF {\n",
        ],
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
           \tif _v, ok := s.Text().Get(); ok {\n\
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
           \tif _v, ok := s.Alpha().Get(); ok {\n\
           \t\treturn uint8(_v.Len())\n\
           \t}\n\
           \tif _v, ok := s.Beta().Get(); ok {\n\
           \t\treturn uint8(_v.Len())\n\
           \t}\n\
           \tif _s, ok := s.SpanLen().Get(); ok {\n\
           \t\treturn _s\n\
           \t}\n\
           \treturn 0\n\
           }\n"],
    );
}

/// A body is presented as its codec's source interface and held as one,
/// written by its codec's own `EncodeSource`; a body whose encode can refuse
/// is first counted over a sink that keeps nothing, which returns its refusal
/// before the first byte and gives the length that measures it.
///
/// Turns red where `go_source::render` presents a body as its codec's value
/// rather than its source, where the zero value presents no body, where
/// `count_body` stops counting over `codec.CountingSink`, where the encoded
/// length is read off the source, or where the decode's literal holds the
/// value it read rather than its address (`made`).
#[test]
fn a_body_is_presented_as_its_source_and_its_encoded_length_is_counted() {
    assert!(unlowered("codec_length_shared").is_empty());
    assert!(unlowered("codec_encoded_length_checked").is_empty());
    let text = resource_go("codec_encoded_length_checked");
    holds(
        "codec_encoded_length_checked",
        &text,
        &[
            "\tInner() codec_length_shared.CodecLengthSharedSource\n",
            "func (s *CodecEncodedLengthChecked) Inner() codec_length_shared.CodecLengthSharedSource {\n\
             \tif s.inner == nil {\n\
             \t\treturn &codec_length_shared.CodecLengthShared{}\n\
             \t}\n\
             \treturn s.inner\n\
             }\n",
            "\tvar _kInner codec.CountingSink\n\
             \tif err := codec_length_shared.EncodeSource(src.Inner(), &_kInner); err != nil {\n\
             \t\treturn err\n\
             \t}\n",
            "uint16(_kInner.Position())",
            "\tif err := codec_length_shared.EncodeSource(src.Inner(), w); err != nil {\n",
            "\t\tinner: &Inner,\n",
            "func (s *CodecEncodedLengthChecked) WireInnerSize() uint16 {\n\
             \tvar _k codec.CountingSink\n\
             \t_ = codec_length_shared.EncodeSource(s.Inner(), &_k)\n\
             \treturn uint16(_k.Position())\n\
             }\n",
            "the refusal of a body it holds",
        ],
    );
    holds_none(
        "codec_encoded_length_checked",
        &text,
        &["src.InnerSize()", "\tinnerSize uint16\n"],
    );
}

/// A list is presented as the runtime's `codec.List` of its entries' source
/// interface, held to the bound type the codec names for it, its count
/// derived from how many entries it holds, and each entry written by its
/// codec's own encode as the list's `iter.Seq` yields it; a chain that ends
/// with its scope is walked as a list is.
///
/// Turns red where `go_source::render` presents a list as a slice of values,
/// where the decode makes the list without `codec.MakeList` over the entries'
/// addresses, where the count is read off the source, or where the walk
/// ranges over anything but `All()`.
#[test]
fn a_list_is_held_to_its_bound_walked_by_its_sequence_and_counted() {
    assert!(unlowered("codec_declared_bounds").is_empty());
    let text = resource_go("codec_declared_bounds");
    holds(
        "codec_declared_bounds",
        &text,
        &[
            "func (CodecDeclaredBoundsItemsBound) Most() int { return 2 }\n",
            "\tItems() codec.List[CodecDeclaredBoundsItemsBound, codec_repeat_elem.CodecRepeatElemSource]\n",
            "\t_dCount := uint8(src.Items().Len())\n",
            "\tfor _e := range src.Items().All() {\n\
             \t\tif err := codec_repeat_elem.EncodeSource(_e, w); err != nil {\n\
             \t\t\treturn err\n\
             \t\t}\n\
             \t}\n",
            "\t_lItems, err := codec.MakeList[CodecDeclaredBoundsItemsBound](codec.SourcesOf(Items, \
             func(e *codec_repeat_elem.CodecRepeatElem) codec_repeat_elem.CodecRepeatElemSource { return e }))\n",
            "\t\titems: _lItems,\n",
            "func (s *CodecDeclaredBounds) WireCount() uint8 {\n\treturn uint8(s.Items().Len())\n}\n",
        ],
    );
    holds_none(
        "codec_declared_bounds",
        &text,
        &["src.Count()", "\tcount uint8\n"],
    );
    assert!(unlowered("codec_tlv_chain_basic").is_empty());
    let text = resource_go("codec_tlv_chain_basic");
    holds(
        "codec_tlv_chain_basic",
        &text,
        &["\tfor _e := range src.Extensions().All() {\n\
           \t\tif err := codec_tlv_entry.EncodeSource(_e, w); err != nil {\n"],
    );
}

/// A codec whose one-bit flags a parent may set takes them in a struct of
/// its own, `{Codec}Overrides`, by an encode that folds each one it sets into
/// its carrier last; `EncodeSource` sets none. A chain that ends on its
/// entries' flag sets it on every entry but the last by that struct.
///
/// Turns red where `encode_source` stops declaring the overrides or folding
/// them, where `EncodeSource` stops handing an empty struct, where
/// `positional_forms` names a chain's end flag or its walk again, or where
/// `walk_entries` sets the flag on the last entry too or counts the entries
/// left other than down from the list's length.
#[test]
fn a_chain_sets_its_entries_end_flag_by_the_entry_codecs_overrides() {
    assert!(unlowered("codec_chain_depth_entry").is_empty());
    let entry = resource_go("codec_chain_depth_entry");
    holds(
        "codec_chain_depth_entry",
        &entry,
        &[
            "type CodecChainDepthEntryOverrides struct {\n\
             \t// HeaderZ is the `Z` flag of `header`, where a parent sets it.\n\
             \tHeaderZ optional.Of[bool]\n\
             }\n",
            "func EncodeSource(src CodecChainDepthEntrySource, w codec.SceSink) error {\n\
             \treturn EncodeSourceOverriding(src, w, CodecChainDepthEntryOverrides{})\n\
             }\n",
            "func EncodeSourceOverriding(src CodecChainDepthEntrySource, w codec.SceSink, \
             overrides CodecChainDepthEntryOverrides) error {\n\
             \t_cHeader := src.Header()\n\
             \tif _o, ok := overrides.HeaderZ.Get(); ok {\n\
             \t\t_cHeader &^= 0x80\n\
             \t\tif _o {\n\
             \t\t\t_cHeader |= 0x80\n\
             \t\t}\n\
             \t}\n",
        ],
    );
    assert!(unlowered("codec_chain_depth_plain").is_empty());
    let chain = resource_go("codec_chain_depth_plain");
    holds(
        "codec_chain_depth_plain",
        &chain,
        &["\t{\n\
           \t\t_leftExtensions := src.Extensions().Len()\n\
           \t\tfor _e := range src.Extensions().All() {\n\
           \t\t\t_leftExtensions--\n\
           \t\t\tif err := codec_chain_depth_entry.EncodeSourceOverriding(_e, w, \
           codec_chain_depth_entry.CodecChainDepthEntryOverrides{HeaderZ: \
           optional.Some(_leftExtensions != 0)}); err != nil {\n\
           \t\t\t\treturn err\n\
           \t\t\t}\n\
           \t\t}\n\
           \t}\n"],
    );
}

/// A chain hands each entry, for an input it binds to the entry before,
/// whether that entry was of the kind the input waits for, clear for the
/// first, both where it counts the entries for their refusal and where it
/// writes them.
///
/// Turns red where `positional_forms` names a walk that hands its entries a
/// flag, where `walk_entries` stops keeping the entry before's kind or keeps
/// it before the entry is written, where `entry_identifier` reads the
/// identifier other than off the entry's carrier, or where `body_encode`
/// hands an input bound to nothing other than the walk's local.
#[test]
fn a_chain_hands_each_entry_what_the_one_before_it_was() {
    assert!(unlowered("codec_chain_handed_entry").is_empty());
    assert!(unlowered("codec_chain_handed").is_empty());
    let text = resource_go("codec_chain_handed");
    holds(
        "codec_chain_handed",
        &text,
        &[
            "\t{\n\
             \t\t_prevAfterMarker := byte(0)\n\
             \t\t_leftEntries := src.Entries().Len()\n\
             \t\tfor _e := range src.Entries().All() {\n\
             \t\t\t_leftEntries--\n\
             \t\t\tif err := codec_chain_handed_entry.EncodeSourceOverriding(_e, w, \
             _prevAfterMarker, codec_chain_handed_entry.CodecChainHandedEntryOverrides{HeaderZ: \
             optional.Some(_leftEntries != 0)}); err != nil {\n\
             \t\t\t\treturn err\n\
             \t\t\t}\n\
             \t\t\t_prevAfterMarker = 0\n\
             \t\t\tif ((uint64(_e.Header()) >> 0) & 0xF) == 0x4 {\n\
             \t\t\t\t_prevAfterMarker = 1\n\
             \t\t\t}\n\
             \t\t}\n\
             \t}\n",
            "codec_chain_handed_entry.EncodeSourceOverriding(_e, &_kEntries, _prevAfterMarker, ",
        ],
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
/// Turns red where `PrintedCodec::go_rebuilt` hands `Make{Codec}` every
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
/// Turns red where `PrintedCodec::go_rebuilt` makes a bounded value of
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
/// `PrintedCodec::go_rebuilt` does not find, which leaves the assign as a write of
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
/// Turns red where `PrintedCodec::go_read` reads a derived element by a method of
/// the field's own name, which the value does not have, or where
/// `stateful_import_field_renames` stops reading a printed codec's element
/// through `PrintedCodec::go_read`.
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

/// A codec named `name` that holds `codec_chain_handed_entry`, which takes
/// `after_marker` and does not say it, bound as `bind` says, measured by
/// `size` and declared after `before` (the codec's own inputs and fields
/// before the body) and followed by `after`.
fn counted_handed_entry(name: &str, inputs: &str, bind: &str, before: &str, after: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:encode-may-fail="true" sce:default-endian="big" name="{name}">
  <sce:import src="codec_chain_handed_entry.scxml" kind="codec" as="codec_chain_handed_entry">
    <sce:flag-bind input="after_marker" source="{bind}"/>
  </sce:import>
{inputs}  <datamodel>
{before}    <sce:embed id="entry" type="codec_chain_handed_entry" sce:byte="2"
               sce:length-from="size"/>
{after}  </datamodel>
</scxml>
"#
    )
}

/// A codec that counts a body handed a flag input of its own: the codec takes
/// the input, which the value does not hold.
fn counted_body_handed_an_input() -> String {
    counted_handed_entry(
        "go_counted_body_handed_an_input",
        "  <sce:flag-inputs>\n    <sce:flag-input name=\"marker\" width=\"1\"/>\n  </sce:flag-inputs>\n",
        "marker",
        "    <sce:field id=\"head\" sce:type=\"uint8\" sce:byte=\"0\" sce:bit-size=\"8\"/>\n\
         \x20   <sce:field id=\"size\" sce:type=\"uint8\" sce:byte=\"1\" sce:bit-size=\"8\"/>\n",
        "",
    )
}

/// The length of a body handed a flag input of the codec's own is answered
/// by a method that takes the input, as the encode is handed it, and the
/// encode counts the body handed the input it is handed.
///
/// Turns red where `wire_numbers` leaves out a number that counts a body
/// handed an input, or where `wire_methods` stops declaring the inputs
/// `wire_inputs` names as the method's parameters.
#[test]
fn the_length_of_a_body_handed_an_input_is_answered_by_a_method_taking_it() {
    let name = "go_counted_body_handed_an_input";
    let text = go_of_document(name, &counted_body_handed_an_input())
        .unwrap_or_else(|e| panic!("{name} generates on Go: {e}"));
    holds(
        name,
        &text,
        &[
            "\nfunc EncodeSource(src GoCountedBodyHandedAnInputSource, w codec.SceSink, marker byte) error {\n",
            "\tif err := codec_chain_handed_entry.EncodeSource(src.Entry(), &_kEntry, marker); err != nil {\n",
            "func (s *GoCountedBodyHandedAnInput) WireSize(marker byte) uint8 {\n\
             \tvar _k codec.CountingSink\n\
             \t_ = codec_chain_handed_entry.EncodeSource(s.Entry(), &_k, marker)\n",
        ],
    );
}

/// A reader that holds a value of a printed codec, and none of the inputs its
/// encode is handed, is not offered a number the value answers only with
/// them: a validator that reads it is refused where it is generated.
///
/// Turns red where `PrintedCodec::reads` answers every element, where
/// `validate_and_enrich_imports` offers every field of a printed Go codec as
/// a member, or where `PrintedCodec::go_read` reads the number without the
/// inputs.
#[test]
fn a_reader_is_not_offered_a_length_that_takes_the_encodes_inputs() {
    let dir = tempfile::tempdir().expect("a directory for the documents");
    std::fs::copy(
        resources().join("codec_chain_handed_entry.scxml"),
        dir.path().join("codec_chain_handed_entry.scxml"),
    )
    .expect("the entry codec is copied");
    std::fs::write(
        dir.path().join("go_counted_body_handed_an_input.scxml"),
        counted_body_handed_an_input(),
    )
    .expect("the codec is written");
    let validator = |plausibility: &str| {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="validator" version="1.0">
  <sce:import src="go_counted_body_handed_an_input.scxml" kind="codec" as="frame"/>
  <datamodel>
    <data id="n" sce:type="uint8" sce:direction="in"
          sce:range-min="0" sce:range-max="32"/>
    <data id="valid" sce:type="bool" sce:direction="out"
          sce:plausibility="{plausibility}"/>
  </datamodel>
</scxml>
"#
        )
    };
    let options = options();
    let compile = |plausibility: &str| {
        compile_forge_with_imports(
            &validator(plausibility),
            DocumentLabel::symmetric("go_validator_reads_handed_length"),
            Language::Go,
            dir.path(),
            &options,
        )
        .map(|_| ())
        .map_err(|e| format!("{:?}", e.error))
    };
    // The value answers its own element.
    compile("frame.head === n").expect("a validator reads an element the value presents");
    let refusal = compile("frame.size === n")
        .expect_err("a length that takes the encode's inputs is no member of the value");
    assert!(
        refusal.contains("size"),
        "the refusal names the member: {refusal}"
    );
}

/// The length of a body handed a flag of a carrier the encode derives is
/// counted once the carrier is folded, the body handed the flag as the frame
/// carries it; the value answers the carrier as the frame carries it by a
/// method of its own, which the length's method hands the body.
///
/// Turns red where `positional_forms` names the shape again, where
/// `encode_source` counts the body before the carriers are folded (the call
/// would read `src.Flags()`), or where `wire_methods` hands the body the
/// carrier the value presents rather than `wire_carrier`'s.
#[test]
fn the_length_of_a_body_handed_a_derived_flag_is_counted_after_the_fold() {
    let name = "go_counted_body_handed_a_derived_flag";
    let document = counted_handed_entry(
        name,
        "",
        "flags.has_x",
        "    <sce:flags id=\"flags\" sce:type=\"uint8\" sce:byte=\"0\" sce:bit-size=\"8\">\n\
         \x20     <sce:flag name=\"has_x\" bit=\"0\"/>\n\
         \x20   </sce:flags>\n\
         \x20   <sce:field id=\"size\" sce:type=\"uint8\" sce:byte=\"1\" sce:bit-size=\"8\"/>\n",
        "    <sce:field id=\"x\" sce:type=\"uint8\" sce:byte=\"3\" sce:bit-size=\"8\"\n\
         \x20              sce:present-if=\"flags.has_x\"/>\n",
    );
    let text =
        go_of_document(name, &document).unwrap_or_else(|e| panic!("{name} generates on Go: {e}"));
    holds(
        name,
        &text,
        &[
            "\t_cFlags := src.Flags()\n\
             \t_cFlags &^= 0x1\n\
             \tif src.X().IsPresent() {\n\
             \t\t_cFlags |= 0x1\n\
             \t}\n\
             \tvar _kEntry codec.CountingSink\n\
             \tif err := codec_chain_handed_entry.EncodeSource(src.Entry(), &_kEntry, \
             byte((_cFlags >> 0) & 0x1)); err != nil {\n",
            "\t_ = codec_chain_handed_entry.EncodeSource(s.Entry(), &_k, \
             byte((s.WireFlags() >> 0) & 0x1))\n",
            "func (s *GoCountedBodyHandedADerivedFlag) WireFlags() uint8 {\n\
             \t_c := s.Flags()\n\
             \t_c &^= 0x1\n\
             \tif s.X().IsPresent() {\n\
             \t\t_c |= 0x1\n\
             \t}\n\
             \treturn _c\n\
             }\n",
        ],
    );
    holds_none(name, &text, &["byte((src.Flags() >> 0)"]);
}

/// A variant whose tag a field of its own holds is presented as the arm the
/// value holds, a value of an interface only the arm types of its package
/// satisfy; the encode derives the tag from the arm, the catch-all's from
/// the tag it holds, and writes the arm by its codec's `EncodeSource`. The
/// catch-all holds its tag as a type made only of a tag no arm names
/// (docs/adr/0065 §6 R3), and the value answers the tag the frame carries by
/// a method of its own.
///
/// Turns red where `positional_forms` names a variant of printed arms again,
/// where `encode_source` reads the tag off the source or reads the arm twice,
/// where `variant_declarations` stops declaring the sealed interface or an
/// arm's constructor, where `unknown_tag_declaration` admits a tag an arm
/// names, where `render` presents no arm when the value holds none, or where
/// the template's decode builds the arm other than by `go_source.arm_made`.
#[test]
fn a_variant_is_presented_as_the_arm_the_value_holds_and_its_tag_derived() {
    assert!(unlowered("codec_variant_dispatch").is_empty());
    let text = resource_go("codec_variant_dispatch");
    holds(
        "codec_variant_dispatch",
        &text,
        &[
            "type CodecVariantDispatchUnknownTag struct {\n\ttag uint8\n\tset bool\n}\n",
            "func MakeCodecVariantDispatchUnknownTag(tag uint8) (CodecVariantDispatchUnknownTag, bool) {\n\
             \tif tag >= 0x1 && tag <= 0x2 {\n\
             \t\treturn CodecVariantDispatchUnknownTag{}, false\n\
             \t}\n\
             \treturn CodecVariantDispatchUnknownTag{tag: tag, set: true}, true\n\
             }\n",
            "func (t CodecVariantDispatchUnknownTag) Value() uint8 {\n\
             \tif !t.set {\n\
             \t\treturn 0x0\n\
             \t}\n\
             \treturn t.tag\n\
             }\n",
            "type CodecVariantDispatchVariant interface {\n\tisCodecVariantDispatchVariant()\n}\n",
            "type CodecVariantDispatchVariantCodecVariantSessionOpen struct {\n\
             \tbody codec_variant_session_open.CodecVariantSessionOpenSource\n\
             }\n",
            "func MakeCodecVariantDispatchVariantCodecVariantSessionOpen(body \
             codec_variant_session_open.CodecVariantSessionOpenSource) \
             CodecVariantDispatchVariantCodecVariantSessionOpen {\n",
            "func (a CodecVariantDispatchVariantCodecVariantSessionOpen) Body() \
             codec_variant_session_open.CodecVariantSessionOpenSource {\n\
             \tif a.body == nil {\n\
             \t\treturn &codec_variant_session_open.CodecVariantSessionOpen{}\n\
             \t}\n\
             \treturn a.body\n\
             }\n",
            "func (CodecVariantDispatchVariantCodecVariantSessionOpen) isCodecVariantDispatchVariant() {}\n",
            "type CodecVariantDispatchVariantDefault struct {\n\
             \ttag  CodecVariantDispatchUnknownTag\n\
             \tbody codec_variant_session_close.CodecVariantSessionCloseSource\n\
             }\n",
            "func (a CodecVariantDispatchVariantDefault) Tag() CodecVariantDispatchUnknownTag {\n",
            "type CodecVariantDispatchSource interface {\n\tBody() CodecVariantDispatchVariant\n}",
            "type CodecVariantDispatch struct {\n\tbody CodecVariantDispatchVariant\n}",
            "func MakeCodecVariantDispatch(body CodecVariantDispatchVariant) CodecVariantDispatch {\n",
            "func (s *CodecVariantDispatch) Body() CodecVariantDispatchVariant {\n\
             \tif s.body == nil {\n\
             \t\treturn CodecVariantDispatchVariantCodecVariantSessionClose{}\n\
             \t}\n\
             \treturn s.body\n\
             }\n",
            "\t_rBody := src.Body()\n\
             \tvar _tBody uint64\n\
             \tswitch _b := _rBody.(type) {\n\
             \tcase CodecVariantDispatchVariantCodecVariantSessionOpen:\n\
             \t\t_tBody = 0x1\n\
             \tcase CodecVariantDispatchVariantCodecVariantSessionClose:\n\
             \t\t_tBody = 0x2\n\
             \tcase CodecVariantDispatchVariantDefault:\n\
             \t\t_tBody = uint64(_b.Tag().Value())\n\
             \t}\n\
             \t_dMsgId := uint8(_tBody)\n",
            "\tswitch _b := _rBody.(type) {\n\
             \tcase CodecVariantDispatchVariantCodecVariantSessionOpen:\n\
             \t\tif err := codec_variant_session_open.EncodeSource(_b.Body(), w); err != nil {\n\
             \t\t\treturn err\n\
             \t\t}\n",
            "\tdefault:\n\
             \t\tpanic(\"CodecVariantDispatch: the source presents no arm of its variant\")\n\
             \t}\n",
            "func (s *CodecVariantDispatch) WireMsgId() uint8 {\n\
             \tvar _tBody uint64\n\
             \tswitch _b := s.Body().(type) {\n",
            "\t\tbody = MakeCodecVariantDispatchVariantCodecVariantSessionOpen(_arm)\n",
            "\t\tbody = MakeCodecVariantDispatchVariantDefault(\
             CodecVariantDispatchUnknownTag{tag: _tag, set: true}, _arm)\n",
            "\t\tbody: body,\n",
        ],
    );
    // The value holds no tag, and no arm in a field of its own.
    holds_none(
        "codec_variant_dispatch",
        &text,
        &[
            "\tmsgId uint8\n",
            "src.MsgId()",
            "Body CodecVariantDispatchVariant",
            "\tBody: body,",
        ],
    );
}

/// A variant whose caller hands the decode its tag is printed as any variant,
/// and a parent that holds the tag in a flag of its carrier derives the flag
/// from the arm the embedded body holds, each arm type named by the body's
/// package, and folds it into its carrier in place of the bits the value
/// presents.
///
/// Turns red where `positional_forms` names a parent tag again, where
/// `parent_tag_bits` switches on anything but the arm type the body holds,
/// or where `fold_bits` keeps the presented bits.
#[test]
fn a_parent_derives_the_flag_the_arm_of_its_body_fills() {
    assert!(unlowered("codec_source_leaf").is_empty());
    assert!(unlowered("codec_source_parent_tag").is_empty());
    let text = resource_go("codec_source_parent_tag");
    holds(
        "codec_source_parent_tag",
        &text,
        &[
            "\tvar _pHdrKind uint64\n\
             \tswitch src.Body().Body().(type) {\n\
             \tcase codec_source_leaf.CodecSourceLeafVariantCodecVariantSessionOpen:\n\
             \t\t_pHdrKind |= 0x0\n\
             \tcase codec_source_leaf.CodecSourceLeafVariantCodecVariantSessionClose:\n\
             \t\t_pHdrKind |= 0x1\n\
             \t}\n",
            "\t_cHdr := src.Hdr()\n\t_cHdr = (_cHdr &^ 0x1) | (uint8(_pHdrKind & 0x1) << 0)\n",
        ],
    );
}

/// A parent the Go backend keeps on the template's encode, which holds a body
/// the printer prints, reads the arm of the body's variant as the printed
/// body presents it: a type switch over `Body()`, each arm type named by the
/// body's package, and no arm field.
///
/// Turns red where `parent_tag_switch_block_go` reads a printed body's arm
/// by the template's field presence.
#[test]
fn a_template_parent_reads_the_arm_of_a_printed_body_by_its_type() {
    let name = "go_template_parent_of_a_printed_variant";
    let document = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="go_template_parent_of_a_printed_variant">
  <sce:import src="codec_source_leaf.scxml" kind="codec" as="codec_source_leaf">
    <sce:variant-dispatch flag="hdr.kind"/>
  </sce:import>
  <datamodel>
    <sce:flags id="hdr" sce:type="uint8" sce:byte="0" sce:bit-size="8">
      <sce:flag name="kind" bit="0" width="1"/>
    </sce:flags>
    <sce:field id="reading" sce:type="float32" sce:byte="1" sce:bit-size="32"/>
    <sce:embed id="body" type="codec_source_leaf" sce:byte="5"/>
  </datamodel>
</scxml>
"#;
    let text =
        go_of_document(name, document).unwrap_or_else(|e| panic!("{name} generates on Go: {e}"));
    holds(
        name,
        &text,
        &["\tswitch s.Body.Body().(type) {\n\
           \tcase codec_source_leaf.CodecSourceLeafVariantCodecVariantSessionOpen:\n\
           \t\t_derivedHdr |= 0x00\n\
           \tcase codec_source_leaf.CodecSourceLeafVariantCodecVariantSessionClose:\n\
           \t\t_derivedHdr |= 0x01\n\
           \t}\n"],
    );
    holds_none(
        name,
        &text,
        &["func EncodeSource(", "Body.Body.CodecVariant"],
    );
}

/// A variant whose arm's own header holds its tag (a peek byte) derives no
/// tag: the arm writes it. An arm made by default is its codec's value made
/// by its constructor, so the tag its header holds is the arm's.
///
/// Turns red where `variant_declarations` presents a default arm by its
/// codec's zero value where the codec declares a constructor.
#[test]
fn a_peek_byte_variant_writes_its_arm_and_derives_no_tag() {
    assert!(unlowered("codec_variant_default_marker").is_empty());
    let text = resource_go("codec_variant_default_marker");
    holds(
        "codec_variant_default_marker",
        &text,
        &[
            "\t\treturn codec_default_marker_arm_b.NewCodecDefaultMarkerArmB()\n",
            "\t\treturn CodecVariantDefaultMarkerVariantCodecDefaultMarkerArmB{}\n",
            "\t_rBody := src.Body()\n\tswitch _b := _rBody.(type) {\n",
        ],
    );
    holds_none("codec_variant_default_marker", &text, &["_tBody"]);
}

/// A variant whose arm is a codec the Go printer does not print keeps the
/// template's encode, and the printer names the arm.
///
/// Turns red where `positional_forms` stops asking whether each arm's codec
/// is printed.
#[test]
fn a_variant_of_an_arm_the_printer_does_not_print_keeps_the_template_encode() {
    let dir = tempfile::tempdir().expect("a directory for the documents");
    std::fs::write(
        dir.path().join("go_unprinted_arm.scxml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="go_unprinted_arm">
  <datamodel>
    <sce:field id="reading" sce:type="float32" sce:byte="0" sce:bit-size="32"/>
  </datamodel>
</scxml>
"#,
    )
    .expect("the arm is written");
    let variant = dir.path().join("go_variant_of_an_unprinted_arm.scxml");
    std::fs::write(
        &variant,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="go_variant_of_an_unprinted_arm">
  <sce:import src="go_unprinted_arm.scxml" kind="codec" as="go_unprinted_arm"/>
  <datamodel>
    <sce:field id="tag" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:variant tag="tag">
      <sce:arm value="0x01" type="go_unprinted_arm" default="true"/>
      <sce:default type="go_unprinted_arm"/>
    </sce:variant>
  </datamodel>
</scxml>
"#,
    )
    .expect("the variant is written");
    let forms = go_codec_unlowered_file(&variant, &[], &options())
        .expect("the variant reads")
        .expect("the document is a codec");
    assert!(
        forms.iter().any(|f| f
            == "a variant: the arm `go_unprinted_arm` is a codec the Go backend does not print"),
        "the printer names the arm: {forms:?}"
    );
}

/// The fields one flag decides on one side are presented whole, as the
/// optional type of a struct of them; the encode binds each member to a local
/// of its own from the group the source presents, derives the flag from it,
/// and the decode makes the group of what it read where the flag holds.
///
/// Turns red where `positional_forms` names a group of the codec's own flag
/// again, where `bind_group` reads a member off anything but the group, where
/// the presence fold reads the source instead of the member's local, or where
/// the template holds a member in a field of its own (`field.grouped`).
#[test]
fn a_group_of_one_flag_is_presented_whole_and_its_flag_derived() {
    assert!(unlowered("codec_group_enum").is_empty());
    let text = resource_go("codec_group_enum");
    holds(
        "codec_group_enum",
        &text,
        &[
            "func MakeCodecGroupEnumFlagsHasSession(",
            "\treturn CodecGroupEnumFlagsHasSession{state: state, attempts: attempts}\n",
            "func (a CodecGroupEnumFlagsHasSession) Attempts() uint8 {\n\treturn a.attempts\n}\n",
            "\tFlagsHasSession() optional.Of[CodecGroupEnumFlagsHasSession]\n",
            "\tflagsHasSession optional.Of[CodecGroupEnumFlagsHasSession]\n",
            "\t_rFlagsHasSession := src.FlagsHasSession()\n",
            "\tvar _mAttempts optional.Of[uint8]\n",
            "\tif _h, ok := _rFlagsHasSession.Get(); ok {\n\t\t_g := _h\n",
            "\t\t_mAttempts = optional.Some(_g.Attempts())\n",
            "\t_cFlags &^= 0x1\n\tif _mState.IsPresent() {\n\t\t_cFlags |= 0x1\n\t}\n",
            "\tif _mAttempts.IsPresent() {\n",
            "\tvar _gFlagsHasSession optional.Of[CodecGroupEnumFlagsHasSession]\n\
             \tif ((Flags >> 0) & 1) != 0 {\n\
             \t\t_gFlagsHasSession = optional.Some[CodecGroupEnumFlagsHasSession](\
             MakeCodecGroupEnumFlagsHasSession(\
             State.Proven(), Attempts.Proven()))\n\
             \t}\n",
            "\t\tflagsHasSession: _gFlagsHasSession,\n",
        ],
    );
    holds_none(
        "codec_group_enum",
        &text,
        &[
            "src.Attempts()",
            "\tattempts optional.Of[uint8]\n",
            "src.State()",
        ],
    );
}

/// A group of several arms that may hold none is decoded into the optional of
/// the group's type, each arm handed to `optional.Some` under that type: Go
/// infers the type of an arm from its argument, and an optional of an arm is
/// not an optional of the group.
///
/// Turns red where the decode hands an arm to `optional.Some` without the
/// group's type, which `go build` refuses.
#[test]
fn an_arm_of_a_group_that_may_hold_none_is_decoded_as_the_groups_optional() {
    assert!(unlowered("codec_present_if_disjunction").is_empty());
    let text = resource_go("codec_present_if_disjunction");
    holds(
        "codec_present_if_disjunction",
        &text,
        &[
            "\tvar _gFlagsWantsAOrWantsB optional.Of[CodecPresentIfDisjunctionFlagsWantsAOrWantsB]\n",
            "\t\t_gFlagsWantsAOrWantsB = optional.Some[CodecPresentIfDisjunctionFlagsWantsAOrWantsB](\
             MakeCodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsA(Seq.Proven()))\n",
            "\t\t_gFlagsWantsAOrWantsB = optional.Some[CodecPresentIfDisjunctionFlagsWantsAOrWantsB](\
             MakeCodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsAWantsB(Seq.Proven()))\n",
        ],
    );
    holds_none(
        "codec_present_if_disjunction",
        &text,
        &["optional.Some(MakeCodecPresentIfDisjunction"],
    );
}

/// Two lists of one count are one member of the arm that holds both, the
/// runtime's `codec.Aligned` of them, made only of two lists of one count
/// (docs/adr/0065 §6 R5); the count is derived from the first, read off
/// whichever arm the value holds.
///
/// Turns red where `bind_group` reads either list other than out of the pair
/// in the arm that holds both, or where the decode pairs them without
/// `codec.MakeAligned`.
#[test]
fn two_lists_of_one_count_are_held_as_one_aligned_pair() {
    assert!(unlowered("codec_repeat_unconditional_count").is_empty());
    let text = resource_go("codec_repeat_unconditional_count");
    holds(
        "codec_repeat_unconditional_count",
        &text,
        &[
            "\t_rOptionsH := src.OptionsH()\n",
            "\t\t_mLinks = optional.Some(_g.LinksWeights().First())\n\
             \t\t_mWeights = optional.Some(_g.LinksWeights().Second())\n",
            "\t\t_mLinks = optional.Some(_g.Links())\n",
            "\t_dLinksLen := uint8(_mLinks.Proven().Len())\n",
            "_aLinksWeights, err := codec.MakeAligned(_lLinks, _lWeights.Proven())\n\
             \t\tif err != nil {\n\
             \t\t\treturn nil, err\n\
             \t\t}\n",
            "\tlinksWeights codec.Aligned[codec.List[CodecRepeatUnconditionalCountLinksBound, \
             codec_repeat_elem.CodecRepeatElemSource], codec.List[\
             CodecRepeatUnconditionalCountWeightsBound, codec_repeat_elem.CodecRepeatElemSource]]\n",
        ],
    );
    holds_none(
        "codec_repeat_unconditional_count",
        &text,
        &["src.Links()", "src.Weights()"],
    );
}

/// The fields a disjunction of the codec's own flags decides are presented
/// as the arm of a sum the value holds, none where no flag is set, and each
/// flag of the disjunction is derived from the arm (docs/adr/0065 §6 R4c).
///
/// Turns red where `positional_forms` names a flag of a disjunction again, or
/// where `group_gate_bit` sets a flag for an arm that clears it or for the
/// value that holds no arm.
#[test]
fn a_disjunction_derives_each_flag_from_the_arm_the_value_holds() {
    let name = "go_group_of_a_disjunction";
    let document = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="go_group_of_a_disjunction">
  <datamodel>
    <sce:flags id="flags" sce:type="uint8" sce:byte="0" sce:bit-size="8">
      <sce:flag name="wants_a" bit="0"/>
      <sce:flag name="wants_b" bit="1"/>
    </sce:flags>
    <sce:field id="seq" sce:type="uint16" sce:byte="1" sce:bit-size="16"
               sce:present-if="flags.wants_a || flags.wants_b"/>
    <sce:field id="ack" sce:type="uint8" sce:byte="3" sce:bit-size="8"
               sce:present-if="flags.wants_a || flags.wants_b"/>
  </datamodel>
</scxml>
"#;
    let text =
        go_of_document(name, document).unwrap_or_else(|e| panic!("{name} generates on Go: {e}"));
    holds(
        name,
        &text,
        &[
            "\tFlagsWantsAOrWantsB() optional.Of[GoGroupOfADisjunctionFlagsWantsAOrWantsB]\n",
            "type GoGroupOfADisjunctionFlagsWantsAOrWantsB interface {\n",
            "\tvar _aFlagsWantsA uint64\n\
             \tif _h, ok := _rFlagsWantsAOrWantsB.Get(); ok {\n\
             \t\tswitch _h.(type) {\n",
            "\t\t\t_aFlagsWantsA = 1\n",
            "\t_cFlags = (_cFlags &^ 0x1) | (uint8(_aFlagsWantsA & 0x1) << 0)\n",
            "\t_cFlags = (_cFlags &^ 0x2) | (uint8(_aFlagsWantsB & 0x1) << 1)\n",
        ],
    );
    holds_none(name, &text, &["src.Seq()", "src.Ack()"]);
}

/// A codec of a form the Go printer does not lower keeps the template's
/// encode whole: its fields stay exported, and it has no source, constructor
/// or encode over a source.
///
/// Turns red where `go_source::positional_forms` stops naming a group a
/// chain's entries decide or a CBOR map, or where `render_codec` prints a
/// codec it names a form of.
#[test]
fn a_codec_of_a_form_the_printer_does_not_lower_keeps_the_template_encode() {
    for (name, form) in [
        ("codec_chain_has_marker", "a group a chain's entries decide"),
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
        let Ok(Some(forms)) = go_codec_unlowered_file(path, &[], &options()) else {
            continue;
        };
        assert_eq!(
            go_codec_printed_file(path, &[], &options()).ok(),
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
        "codec_length_stated",
        "codec_encoded_length_checked",
        "codec_declared_bounds",
        "codec_tlv_chain_basic",
        "codec_chain_depth_entry",
        "codec_chain_depth_plain",
        "codec_chain_handed_entry",
        "codec_chain_handed",
        "codec_variant_dispatch",
        "codec_variant_peek_basic",
        "codec_variant_default_marker",
        "codec_source_leaf",
        "codec_source_parent_tag",
        "codec_group_enum",
        "codec_repeat_unconditional_count",
    ] {
        assert!(
            printed.iter().any(|stem| stem == name),
            "{name} is printed on Go"
        );
    }
}
