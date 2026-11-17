// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A codec (`sce:kind="codec"`, a frame of fields at byte offsets), written for Swift in part: a
// positional codec of native-width integers is a struct with a `decode` over the runtime's
// `SceCursor` and an `encode` into its `SceSink`, and what the Swift codec does not write yet (a
// repeat, a variant, a string, another encoding) is refused by name and never written without it. A flag carrier is a field like any other, and the flags it names are
// properties of the struct that read and write the bits of it.
//
// The runtime half is `backends/swift/forge-runtime/Sources/SceForgeRuntime/Codec.swift`, held by
// its own tests; this holds what the generator writes against it.

use std::path::{Path, PathBuf};

use sce_build::generator::Language;
use sce_build::{
    compile_forge_file, compile_forge_from_string, DocumentLabel, ForgeCompileOptions,
};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// The Swift source `document` (a path under the repository) is written as, or the refusal it is
/// answered with.
fn swift_of(document: &str) -> Result<String, String> {
    let output = compile_forge_file(
        &repo_root().join(document),
        Language::Swift,
        &[],
        &ForgeCompileOptions::default(),
    )
    .map_err(|e| format!("{e:?}"))?;
    let (_, code) = output
        .files
        .into_iter()
        .next()
        .expect("a document that is written writes a file");
    Ok(code)
}

/// A codec named `flagged` whose first field is a `uint8` flag carrier `header` holding `flags`
/// (the `<sce:flag>` elements), then `extra` (more fields), written to Swift from a string.
fn swift_of_flags(flags: &str, extra: &str) -> Result<String, String> {
    swift_of_datamodel(&format!(
        r#"<sce:flags id="header" sce:type="uint8" sce:byte="0" sce:bit-size="8">{flags}</sce:flags>
    {extra}"#
    ))
}

/// A codec named `flagged` whose `<datamodel>` holds `fields`, written to Swift from a string.
fn swift_of_datamodel(fields: &str) -> Result<String, String> {
    let document = format!(
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="flagged">
  <datamodel>
    {fields}
  </datamodel>
</scxml>"#
    );
    compile_forge_from_string(
        &document,
        DocumentLabel {
            identifier: "flagged",
            diagnostic_label: "flagged.scxml",
        },
        Language::Swift,
    )
    .map_err(|e| format!("{e:?}"))
    .map(|output| {
        output
            .files
            .into_iter()
            .next()
            .expect("a document that is written writes a file")
            .1
    })
}

#[test]
fn a_flag_of_one_bit_is_a_bool_property_over_its_carrier() {
    let code =
        swift_of("tests/forge/resources/codec_flags_basic.scxml").expect("flags are written");
    assert!(
        code.contains("public var header: UInt8 = 0\n"),
        "the carrier is a field like any other: {code}"
    );
    assert!(code.contains("public var reliable: Bool {"), "{code}");
    assert!(code.contains("get { (header & 0x80) != 0 }"), "{code}");
    assert!(
        code.contains("set { header = newValue ? (header | 0x80) : (header & ~0x80) }"),
        "{code}"
    );
    assert!(
        code.contains("storeInteger(header_folded, into: &frame, at: 0, .big)"),
        "the flags do not change the wire: {code}"
    );
    // A one-bit flag a parent may set (the flag a chain's entries end on) is a member of the
    // codec's `Overrides`, folded into its carrier before the carrier is written.
    for (expected, why) in [
        (
            "public var headerReliable: Bool?",
            "the override of a flag is named for its carrier and itself",
        ),
        (
            "into sink: inout Sink, overrides: Overrides = Overrides()) throws(CodecError) {",
            "a caller that sets no flag hands nothing",
        ),
        ("var header_folded = source.header", "the carrier as the value presents it"),
        (
            "if let set = overrides.headerReliable { header_folded = set ? (header_folded | 0x80) : (header_folded & ~0x80) }",
            "a flag the parent sets replaces the value's own",
        ),
    ] {
        assert!(code.contains(expected), "{why}: {expected} in {code}");
    }
}

#[test]
fn a_flag_of_several_bits_is_an_unsigned_property_cut_to_its_width() {
    let code = swift_of("tests/forge/resources/codec_qos_byte.scxml").expect("flags are written");
    assert!(code.contains("public var priority: UInt8 {"), "{code}");
    assert!(
        code.contains("get { UInt8(truncatingIfNeeded: (qos >> 0) & 0x7) }"),
        "{code}"
    );
    assert!(code.contains("public var congestion: UInt8 {"), "{code}");
    assert!(
        code.contains("set { qos = (qos & ~(0x3 << 4)) | (((UInt8(truncatingIfNeeded: newValue)) & 0x3) << 4) }"),
        "a value wider than the flag is cut to it: {code}"
    );
    assert!(code.contains("public var express: Bool {"), "{code}");
}

#[test]
fn the_bits_a_flag_value_sets_are_the_default_of_its_carrier() {
    let code = swift_of_flags(
        r#"<sce:flag name="kind" bit="4" width="4" value="0xA"/><sce:flag name="open" bit="0" value="1"/>"#,
        "",
    )
    .expect("flags are written");
    assert!(code.contains("public var header: UInt8 = 0xA1\n"), "{code}");
}

#[test]
fn a_flag_that_is_a_reserved_word_is_refused_when_the_document_is_validated() {
    // The property is spelled from the name, so the validation that compares the name as each
    // backend spells it is what keeps a reserved word out: Swift's own (`protocol`), every
    // backend's (`default`), and the name that becomes one (`for_` is the property `for`).
    for name in ["protocol", "default", "for_"] {
        let refusal = swift_of_flags(&format!(r#"<sce:flag name="{name}" bit="0"/>"#), "")
            .expect_err(&format!("`{name}` is not a property"));
        assert!(
            refusal.contains("ReservedCodeIdentifier"),
            "{name}: {refusal}"
        );
    }
}

#[test]
fn a_flag_named_for_a_member_of_the_struct_is_refused() {
    // The member Swift's struct has that no other backend's codec has.
    let refusal = swift_of_flags(r#"<sce:flag name="frame_length" bit="0"/>"#, "")
        .expect_err("a flag is not the frame length");
    assert!(refusal.contains("the member `frameLength`"), "{refusal}");
}

#[test]
fn a_flag_named_for_a_field_is_refused() {
    let refusal = swift_of_flags(
        r#"<sce:flag name="length" bit="0"/>"#,
        r#"<sce:field id="length" sce:type="uint8" sce:byte="1" sce:bit-size="8"/>"#,
    )
    .expect_err("a flag is not a field");
    assert!(refusal.contains("the field 'length'"), "{refusal}");
}

#[test]
fn two_flags_that_are_one_name_are_refused_when_the_document_is_validated() {
    let refusal = swift_of_flags(
        r#"<sce:flag name="more_data" bit="0"/><sce:flag name="moreData" bit="1"/>"#,
        "",
    )
    .expect_err("two flags are not one");
    assert!(refusal.contains("CollidingCodeIdentifier"), "{refusal}");
}

#[test]
fn a_codec_of_integers_is_a_struct_with_a_decode_and_an_encode() {
    let code = swift_of("tests/forge/resources/codec_simple_frame.scxml")
        .expect("a codec of integers is written");
    assert!(
        code.contains(
            "public struct CodecSimpleFrame: Equatable, Sendable, CodecSimpleFrameSource {"
        ),
        "the value presents its elements through the protocol its encode reads: {code}"
    );
    assert!(
        code.contains("public protocol CodecSimpleFrameSource {\n    var msgId: UInt8 { get }\n    var length: UInt8 { get }\n    var payload: UInt16 { get }\n}"),
        "one read-only property per element, in the document's order: {code}"
    );
    assert!(
        code.contains("public static func encodeSource<Source: CodecSimpleFrameSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {"),
        "the encode is printed over any source of the elements: {code}"
    );
    assert!(
        code.contains("        try Self.encodeSource(self, into: &sink)\n"),
        "the value's own encode is that of itself: {code}"
    );
    assert!(code.contains("public static let frameLength = 4"), "{code}");
    assert!(code.contains("public var msgId: UInt8"), "{code}");
    assert!(code.contains("public var length: UInt8"), "{code}");
    assert!(code.contains("public var payload: UInt16"), "{code}");
    assert!(
        code.contains("public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecSimpleFrame {"),
        "the decode reads a cursor and answers the runtime's error: {code}"
    );
    assert!(
        code.contains(
            "public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {"
        ),
        "the encode writes any sink the runtime has: {code}"
    );
    assert!(
        code.contains("payload: try cursor.peekInteger(UInt16.self, at: 2, .big)"),
        "a field is read at its byte offset in the document's byte order: {code}"
    );
    assert!(
        code.contains("storeInteger(source.payload, into: &frame, at: 2, .big)"),
        "and written at the same place: {code}"
    );
}

#[test]
fn a_codec_is_printed_from_its_plan_where_the_printer_lowers_every_form_and_else_by_its_template() {
    // docs/adr/0081 §6: the printer lowers the plan's forms one at a time, and a codec whose plan
    // holds a form it does not lower keeps its template's encode whole.
    for printed in [
        "tests/forge/resources/codec_simple_frame.scxml",
        "tests/forge/resources/codec_subbyte.scxml",
        "tests/forge/resources/codec_enum_fields.scxml",
        "tests/forge/resources/codec_zenoh_keep_alive.scxml",
        "tests/forge/resources/codec_flags_basic.scxml",
        "tests/forge/resources/codec_optional_field_attributes.scxml",
        "tests/forge/resources/codec_vle_zint_u64.scxml",
        "tests/forge/resources/codec_present_if_basic.scxml",
        "tests/forge/resources/codec_present_if_checked.scxml",
        "tests/forge/resources/codec_tail.scxml",
        "tests/forge/resources/codec_length_ref.scxml",
        "tests/forge/resources/codec_length_ref_dotted_basic.scxml",
        "tests/forge/resources/codec_scout_zid_body.scxml",
        "tests/forge/resources/codec_zenoh_locator.scxml",
        "tests/forge/resources/codec_present_if_string.scxml",
        "tests/forge/resources/codec_zenoh_wireexpr.scxml",
        "tests/forge/resources/codec_zenoh_open_body.scxml",
        "tests/forge/resources/codec_init_cookie_body.scxml",
        "tests/forge/resources/codec_zenoh_timestamp_ext.scxml",
        "tests/forge/resources/codec_zenoh_decl_kexpr.scxml",
        "tests/forge/resources/codec_embed_basic.scxml",
        "tests/forge/resources/codec_repeat_basic.scxml",
        "tests/forge/resources/codec_until_eof_basic.scxml",
        "tests/forge/resources/codec_repeat_present_if_basic.scxml",
        "tests/forge/resources/codec_tlv_chain_basic.scxml",
        "tests/forge/resources/codec_chain_depth_plain.scxml",
        "tests/forge/resources/codec_group_enum.scxml",
        "tests/forge/resources/codec_present_if_disjunction.scxml",
        "tests/forge/resources/codec_chain_flag_entry.scxml",
        "tests/forge/resources/codec_repeat_flag_carrier.scxml",
        "tests/forge/resources/codec_zenoh_value_zbuf.scxml",
        "tests/forge/resources/codec_zenoh_value_entry.scxml",
        "tests/forge/resources/codec_repeat_flag_group.scxml",
        "tests/forge/resources/codec_chain_prev_carrier.scxml",
        "tests/forge/resources/codec_chain_flag_forward.scxml",
        "tests/forge/resources/codec_repeat_unconditional_count.scxml",
        "tests/forge/resources/codec_zenoh_err.scxml",
    ] {
        let code = swift_of(printed).expect("a codec is written");
        assert!(
            code.contains("public static func encodeSource<Source: ")
                && code.contains("public protocol "),
            "{printed} is printed from its plan: {code}"
        );
        assert_eq!(
            code.matches("public func encode<Sink: SceSink>").count(),
            1,
            "{printed} has one encode: {code}"
        );
    }
    for (template, why) in [(
        "tests/forge/resources/codec_chain_has_tagged.scxml",
        "a gate on the entries of a chain",
    )] {
        let code = swift_of(template).expect("a codec is written");
        assert!(
            !code.contains("encodeSource"),
            "{template} keeps its template's encode until the printer lowers {why}: {code}"
        );
    }
}

#[test]
fn a_field_is_read_and_written_in_the_documents_byte_order() {
    let code = swift_of("tests/forge/resources/codec_little_endian.scxml")
        .expect("a codec of integers is written");
    assert!(
        code.contains(
            "public struct CodecLittleEndian: Equatable, Sendable, CodecLittleEndianSource {"
        ),
        "{code}"
    );
    assert!(
        code.contains("value: try cursor.peekInteger(UInt16.self, at: 1, .little)"),
        "{code}"
    );
    assert!(
        code.contains("storeInteger(source.value, into: &frame, at: 1, .little)"),
        "{code}"
    );
    assert!(code.contains("public static let frameLength = 4"), "{code}");
}

#[test]
fn a_frame_is_decoded_whole_or_not_at_all() {
    let code = swift_of("tests/forge/resources/codec_simple_frame.scxml")
        .expect("a codec of integers is written");
    let peeked = code
        .find("_ = try cursor.peekSlice(frameLength)")
        .expect("the length of the frame is checked first");
    let read = code
        .find("try cursor.peekInteger(")
        .expect("the fields are read");
    let advanced = code
        .find("try cursor.advance(frameLength)")
        .expect("the cursor moves");
    assert!(
        peeked < read && read < advanced,
        "a truncated frame answers need-more-bytes before a field is read, and the cursor moves \
         only after the frame is built: {code}"
    );
}

#[test]
fn a_frame_is_written_in_one_write_so_a_bounded_sink_holds_all_of_it_or_none() {
    let code = swift_of("tests/forge/resources/codec_simple_frame.scxml")
        .expect("a codec of integers is written");
    assert_eq!(
        code.matches("sink.writeBytes(").count(),
        1,
        "the frame is laid out in a buffer and handed over once: {code}"
    );
}

#[test]
fn an_encode_reads_a_field_through_its_source_so_a_local_cannot_hide_it() {
    // The printed encode lays the frame out in a local named `frame` and writes to a parameter
    // named `sink`; a field of either name, read bare, would be read as the local or the parameter.
    let code = swift_of_datamodel(
        r#"<sce:field id="frame" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
           <sce:field id="sink" sce:type="uint8" sce:byte="1" sce:bit-size="8"/>
           <sce:field id="low" sce:type="uint8" sce:byte="2" sce:bit-offset="0" sce:bit-size="4"/>"#,
    )
    .expect("fields named for the encode's own names are written");
    assert!(
        code.contains("storeInteger(source.frame, into: &frame, at: 0, .big)"),
        "{code}"
    );
    assert!(
        code.contains("storeInteger(source.sink, into: &frame, at: 1, .big)"),
        "{code}"
    );
    assert!(
        code.contains("frame[2] |= (source.low & 0xF) << 0"),
        "{code}"
    );
}

#[test]
fn a_tail_is_the_bytes_to_the_end_of_the_frame() {
    let code = swift_of("tests/forge/resources/codec_tail.scxml").expect("a tail is written");
    // A byte string is held in the bounded type its plan gives it (docs/adr/0065 §6 R1), which a
    // decode makes through the door that refuses what the bound does not hold.
    assert!(
        code.contains("public var payload: BoundedBytes<CodecTail.PayloadBound> = .fewest"),
        "{code}"
    );
    assert!(
        code.contains("public enum PayloadBound: SceByteBound {"),
        "the bound is a type of the codec's own: {code}"
    );
    assert!(
        code.contains("public static let frameLength = 2"),
        "the fixed part is the bytes before the tail: {code}"
    );
    assert!(
        code.contains("var reader = cursor"),
        "the frame is read through a copy of the cursor: {code}"
    );
    assert!(
        code.contains(
            "let payload = try BoundedBytes<CodecTail.PayloadBound>(decoding: Array(try reader.readSlice(reader.remaining)))"
        ),
        "{code}"
    );
    assert!(code.contains("cursor = reader"), "{code}");
    assert!(
        code.contains("var frame = [UInt8](repeating: 0, count: 2)"),
        "the fixed part is written into zeros of its bytes: {code}"
    );
    assert!(
        code.contains("source.payload.append(to: &frame)"),
        "the encode appends the bytes after the fixed part: {code}"
    );
}

#[test]
fn a_variable_length_integer_is_read_and_written_as_a_chain() {
    let code =
        swift_of("tests/forge/resources/codec_vle_zint_u64.scxml").expect("a VLE is written");
    assert!(code.contains("public var value: UInt64 = 0"), "{code}");
    assert!(
        code.contains("let value = try reader.readVle(UInt64.self)"),
        "{code}"
    );
    assert!(
        code.contains("return VleZintU64(value: value)"),
        "the value is made of the locals the frame was read into: {code}"
    );
    // A number written in turn is printed from the plan (docs/adr/0081 §6): the frame grows as
    // each field is appended, read off the source.
    assert!(
        code.contains("appendVle(source.value, to: &frame)"),
        "{code}"
    );
    assert!(code.contains("var frame: [UInt8] = []"), "{code}");
    assert!(
        code.contains("public static let frameLength = 1"),
        "a chain takes one byte at the least: {code}"
    );
}

#[test]
fn a_field_after_a_variable_length_one_begins_where_it_ended() {
    // `time` is a chain, so `zid_size` and the bytes it counts have no byte of their own.
    let code =
        swift_of("tests/forge/resources/codec_ext_timestamp.scxml").expect("a VLE is written");
    let time = code
        .find("let time = try reader.readVle")
        .expect("the chain is read");
    let size = code
        .find("let zid_size = try reader.readInteger")
        .expect("the size is read");
    let zid = code
        .find("let zid = try BoundedBytes<")
        .expect("the bytes are read");
    assert!(
        time < size && size < zid,
        "the fields are read in the order of the document: {code}"
    );
    let fixed_after = swift_of("tests/forge/resources/codec_fixed_after_lengthref.scxml")
        .expect("a fixed field after a length-ref is written");
    let bytes = fixed_after
        .find("let payload = ")
        .expect("the bytes are read");
    let crc = fixed_after
        .find("let crc32 = try reader.readInteger")
        .expect("the crc is read");
    assert!(
        bytes < crc,
        "a field after the bytes is read after them: {fixed_after}"
    );
}

#[test]
fn a_length_ref_takes_its_count_from_an_unsigned_field_and_its_arithmetic() {
    let code = swift_of("tests/forge/resources/codec_length_ref_uint16_be.scxml")
        .expect("a length-ref is written");
    assert!(
        code.contains("(decoding: Array(try reader.readSlice(Int(clamping: payload_len))))"),
        "{code}"
    );
    let arith = swift_of("tests/forge/resources/codec_scout_zid_body.scxml")
        .expect("a length-ref with arithmetic is written");
    assert!(
        arith.contains("Int(clamping: zid_len_m1) &+ (1)"),
        "{arith}"
    );
}

#[test]
fn a_byte_string_is_held_to_the_bound_its_document_declares() {
    let tail = swift_of_datamodel(
        r#"<sce:field id="body" sce:type="bytes" sce:byte="0" sce:bit-size="tail" sce:max-size="8"/>"#,
    )
    .expect("a bounded tail is written");
    assert!(
        tail.contains("if reader.remaining > 8 {\n            throw CodecError.lengthOutOfRange\n        }\n        let body = try BoundedBytes<Flagged.BodyBound>(decoding: Array(try reader.readSlice(reader.remaining)))"),
        "a decode refuses the bytes that are left before it cuts them: {tail}"
    );
    // A value past the bound is no value of the field's type (docs/adr/0065 §6 R1), so the encode
    // has nothing of it to refuse.
    assert!(
        tail.contains(
            "public enum BodyBound: SceByteBound {\n        public static let least = 0\n        public static let most: Int? = 8\n    }"
        ),
        "the bound is the type's: {tail}"
    );
    assert!(
        !tail.contains("throw CodecError.lengthOutOfRange\n        }\n        var frame"),
        "the encode refuses no bound its type holds: {tail}"
    );
    let counted = swift_of_datamodel(
        r#"<sce:field id="tag_len" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
           <sce:field id="tag" sce:type="bytes" sce:byte="1" sce:bit-size="length-ref" sce:length-field="tag_len" sce:length-arith="-1" sce:max-size="4"/>"#,
    )
    .expect("a bounded length-ref is written");
    assert!(
        counted.contains("if tag_len < 1 || tag_len > 5 {"),
        "the carried value is judged in its own type, with the arithmetic folded into the bound, so a carried 0 that says -1 is refused and not wrapped: {counted}"
    );
}

#[test]
fn a_text_is_the_string_its_utf8_bytes_spell_and_is_measured_in_those_bytes() {
    let code = swift_of_datamodel(
        r#"<sce:field id="n" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
           <sce:field id="name" sce:type="string" sce:byte="1" sce:bit-size="length-ref" sce:length-field="n" sce:max-size="8"/>"#,
    )
    .expect("a text is written");
    assert!(
        code.contains("\n    public var name: BoundedText<Flagged.NameBound> = .fewest\n"),
        "a field is declared on a line of its own, after its comment: {code}"
    );
    assert!(
        code.contains(
            "let name = try BoundedText<Flagged.NameBound>(decoding: try reader.readString(Int(clamping: n)))"
        ),
        "a decode reads the bytes as UTF-8 text, refusing bytes that are not: {code}"
    );
    assert!(
        code.contains("source.name.append(to: &frame)"),
        "an encode writes the UTF-8 bytes of the text: {code}"
    );
    // The length is derived from the bytes on the wire, not the characters (docs/adr/0081 §3), and
    // the value holds none of its own.
    assert!(
        code.contains("let n_derived: UInt8 = UInt8(source.name.byteCount)"),
        "{code}"
    );
    assert!(
        code.contains("appendInteger(n_derived, to: &frame, .big)")
            || code.contains("storeInteger(n_derived, into: &frame, at: 0, .big)"),
        "the derived length is what is written: {code}"
    );
    assert!(
        !code.contains("public var n:"),
        "the value holds no length of its own: {code}"
    );
}

#[test]
fn a_gated_text_reads_a_length_that_shares_its_gate_through_the_value_it_has_there() {
    let code = swift_of("tests/forge/resources/codec_present_if_string.scxml")
        .expect("a gated text with a gated length is written");
    assert!(
        code.contains("public var text: BoundedText<CodecPresentIfString.TextBound>? = nil"),
        "{code}"
    );
    // The length is derived where the text is there, and written where its gate holds.
    assert!(
        code.contains("let text_len_derived: UInt8 = source.text != nil ? UInt8((source.text?.byteCount ?? 0)) : 0"),
        "{code}"
    );
    assert!(
        code.contains("try reader.readString(Int(clamping: (text_len ?? 0)))"),
        "{code}"
    );
}

#[test]
fn a_length_that_is_on_the_wire_under_another_gate_than_its_field_is_refused() {
    let refusal = swift_of_datamodel(
        r#"<sce:flags id="header" sce:type="uint8" sce:byte="0" sce:bit-size="8"><sce:flag name="has_n" bit="0"/></sce:flags>
           <sce:field id="n" sce:type="uint8" sce:byte="1" sce:bit-size="8" sce:present-if="header.has_n"/>
           <sce:field id="body" sce:type="bytes" sce:byte="2" sce:bit-size="length-ref" sce:length-field="n" sce:max-size="8"/>"#,
    )
    .expect_err("a length gated otherwise than its field is not written");
    assert!(
        refusal.contains("on the wire only on a condition the field is not"),
        "{refusal}"
    );
}

#[test]
fn a_variant_is_the_arm_its_tag_selects_with_the_tag_the_frame_said_for_the_catch_all() {
    let code = swift_of("tests/forge/resources/codec_variant_dispatch.scxml")
        .expect("a variant over a field is written");
    assert!(
        code.contains("    case codecVariantSessionOpen(A0)\n    case codecVariantSessionClose(A1)\n    case `default`(tag: CodecVariantDispatchUnknownTag, body: D)\n"),
        "{code}"
    );
    // docs/adr/0065 §6 R3: the catch-all's tag admits no tag an arm names; a whole byte holds no
    // tag past its bits.
    assert!(
        code.contains("public struct CodecVariantDispatchUnknownTag: Equatable, Hashable, Sendable {\n    /// The tag.\n    public let value: UInt8\n"),
        "{code}"
    );
    assert!(
        code.contains("        switch value {\n        case 0x1...0x2: return false\n        default: return true\n        }"),
        "{code}"
    );
    assert!(
        code.contains("\n    public var body: CodecVariantDispatchVariant = .codecVariantSessionClose(CodecVariantSessionClose())\n"),
        "the arm the document marks as its default is what a value that has read no frame holds: {code}"
    );
    assert!(
        code.contains("let body_tag = msg_id\n        switch body_tag {\n        case 0x1:\n            body = .codecVariantSessionOpen(try CodecVariantSessionOpen.decode(&reader))\n        case 0x2:"),
        "{code}"
    );
    assert!(
        code.contains("        default:\n            body = .default(tag: CodecVariantDispatchUnknownTag(admitted: body_tag), body: try CodecVariantSessionClose.decode(&reader))"),
        "{code}"
    );
    // Printed from its plan (docs/adr/0081 §3): the tag is derived from the arm the value holds,
    // and the arm is written by its own codec's encode over the source its body is.
    assert!(
        code.contains(
            "        let msg_id_derived: UInt8 = UInt8(truncatingIfNeeded: source.body.tag)\n"
        ),
        "the encode derives the tag from the arm: {code}"
    );
    assert!(
        code.contains("        case .default(_, let arm):\n            try CodecVariantSessionClose.encodeSource(arm, into: &body_arm)\n"),
        "an encode writes the arm that is held: {code}"
    );
    assert!(
        code.contains(
            "public enum CodecVariantDispatchArms<A0: CodecVariantSessionOpenSource, A1: CodecVariantSessionCloseSource, D: CodecVariantSessionCloseSource> {"
        ) && code.contains(
            "public typealias CodecVariantDispatchVariant = CodecVariantDispatchArms<CodecVariantSessionOpen, CodecVariantSessionClose, CodecVariantSessionClose>"
        ),
        "a source presents each arm's body as a source of the arm's codec: {code}"
    );
    assert!(
        code.contains("public static let frameLength = 1 + Swift.min(CodecVariantSessionOpen.frameLength, CodecVariantSessionClose.frameLength)"),
        "the fewest bytes are the tag's and the shortest arm's: {code}"
    );
}

#[test]
fn a_variant_over_the_bits_of_a_flag_reads_them_shifted_down_and_masked() {
    let code = swift_of("tests/forge/resources/codec_zenoh_push_body.scxml")
        .expect("a variant over a flag is written");
    assert!(
        code.contains("let body_tag = (header >> 0) & 0x1F"),
        "{code}"
    );
    assert!(
        code.contains("body = .default(tag: CodecZenohPushBodyUnknownTag(admitted: UInt8(truncatingIfNeeded: body_tag)), body: try CodecZenohPut.decode(&reader))"),
        "the tag the catch-all holds is the bits, in the smallest type that holds them: {code}"
    );
    assert!(
        code.contains(
            "        case 0x1...0x2: return false\n        default: return value <= 0x1F\n"
        ),
        "the catch-all's tag holds no tag past the flag's five bits: {code}"
    );
}

#[test]
fn a_variant_over_a_peeked_byte_reads_the_tag_and_leaves_the_byte_for_the_arm() {
    let code = swift_of("tests/forge/resources/codec_variant_peek_basic.scxml")
        .expect("a variant over a peeked byte is written");
    assert!(
        code.contains(
            "let body_tag = try (reader.peekInteger(UInt8.self, at: 0, .big) >> 0) & 0x1\n        switch body_tag {\n        case 0x0:"
        ),
        "the tag is read without moving the cursor, so the arm reads the byte as its own header: {code}"
    );
    assert!(
        code.contains(
            "default:\n            body = .codecPeekArmB(try CodecPeekArmB.decode(&reader))"
        ),
        "no catch-all: the last arm stands for what is left: {code}"
    );
}

#[test]
fn an_embedded_variant_whose_arm_the_embedder_drives_is_written_both_ways() {
    // The importer declares a dispatch that writes the arm into one of its own flags: the encode
    // writes the tag of the arm the child holds into those bits of the carrier, keeping every other
    // bit of the stored value, whether the child finds its own tag on the wire (as this one does) or
    // is handed it.
    let code = swift_of("tests/forge/resources/codec_embed_variant_dispatch.scxml")
        .expect("an embed whose arm is driven into a flag is written");
    assert!(
        code.contains("        hdr_kind_filled |= UInt64(source.body.body.tag) & 0x3\n"),
        "the held arm's tag is read off the child's arms enum: {code}"
    );
    assert!(
        code.contains(
            "        hdr_folded = (hdr_folded & ~0x3) | ((UInt8(truncatingIfNeeded: hdr_kind_filled) & 0x3) << 0)\n"
        ),
        "the encode writes the held arm's tag into the flag: {code}"
    );
}

#[test]
fn a_flag_input_is_a_parameter_of_decode_and_encode_that_a_gate_reads() {
    let code = swift_of("tests/forge/resources/codec_zenoh_wireexpr.scxml")
        .expect("a codec with a flag input is written");
    assert!(
        code.contains(
            "public static func decode(_ cursor: inout SceCursor, N: UInt8) throws(CodecError) -> CodecZenohWireexpr"
        ),
        "{code}"
    );
    assert!(
        code.contains("        if (N & 0x1) != 0 {\n            suffix_len = "),
        "a decode reads a field gated by the input only when the input is set: {code}"
    );
    // An input the value does not say is a parameter of the encode too.
    let given = swift_of("tests/forge/resources/codec_chain_handed_entry.scxml")
        .expect("a codec handed an input its value does not say is written");
    assert!(
        given.contains("into sink: inout Sink, after_marker: UInt8")
            && given.contains(
                "public func encode<Sink: SceSink>(into sink: inout Sink, after_marker: UInt8) throws(CodecError)"
            ),
        "{given}"
    );
}

#[test]
fn an_input_the_value_says_is_read_off_it_and_is_no_parameter_of_the_encode() {
    // docs/adr/0065 §6 R4a: `suffix` is gated on `N` alone, so the value says `N` by holding it or
    // not, and the encode reads it off the value where a gate reads it; no value contradicts it, so
    // nothing is refused for it. A parent or a log reads it through `inputN(_:)`.
    let code = swift_of("tests/forge/resources/codec_zenoh_wireexpr.scxml")
        .expect("a codec whose value says its input is written");
    assert!(
        code.contains(
            "public static func encodeSource<Source: CodecZenohWireexprSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {"
        ) && code.contains(
            "public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError)"
        ),
        "the encode is handed no input the value says: {code}"
    );
    let said = code
        .find("let N: UInt8 = source.suffix != nil ? 1 : 0")
        .unwrap_or_else(|| panic!("the encode reads the input off the value: {code}"));
    let gated = code.find("if N != 0 {").unwrap_or_else(|| {
        panic!("the length gated on the input is written where it holds: {code}")
    });
    assert!(said < gated, "the input is read before the gate: {code}");
    assert!(
        code.contains(
            "public static func inputN<Source: CodecZenohWireexprSource>(_ source: Source) -> UInt8 {\n        source.suffix != nil ? 1 : 0\n    }"
        ),
        "a parent reads the input off a source: {code}"
    );
    assert!(
        !code.contains("presentIfMismatch") && !code.contains("suffix_len: suffix_len"),
        "no value contradicts the input, and the value holds no length: {code}"
    );
}

#[test]
fn an_embedder_hands_the_codec_it_embeds_an_input_of_its_own_or_a_bit_of_a_carrier() {
    let forwarded = swift_of("tests/forge/resources/codec_zenoh_decl_kexpr.scxml")
        .expect("an embed that hands an input on is written");
    assert!(
        forwarded.contains("try CodecZenohWireexpr.decode(&reader, N: N)"),
        "{forwarded}"
    );
    // The body is printed from its plan and says `N` itself (docs/adr/0065 §6 R4a), so its encode
    // is handed nothing, and the codec that holds it says `N` by what the body says; the decode is
    // still handed the bit.
    assert!(
        forwarded.contains(
            "        try CodecZenohWireexpr.encodeSource(source.wireexpr, into: &wireexpr_piece)\n"
        ),
        "{forwarded}"
    );
    assert!(
        forwarded.contains(
            "public static func inputN<Source: CodecZenohDeclKexprSource>(_ source: Source) -> UInt8 {\n        CodecZenohWireexpr.inputN(source.wireexpr)\n    }"
        ),
        "{forwarded}"
    );
    // A variant every arm of which says the input its holder binds (docs/adr/0065 §6 R4d) says it
    // by the arm it holds, read through the arm's codec, and hands the arm's encode nothing.
    let arms = swift_of("tests/forge/resources/codec_every_arm_says.scxml")
        .expect("a variant whose arms say their inputs is written");
    assert!(
        arms.contains("    try CodecEveryArmSaysFar.encodeSource(arm, into: &body_arm)\n")
            && arms.contains("    try CodecEveryArmSaysNear.encodeSource(arm, into: &body_arm)\n"),
        "{arms}"
    );
    assert!(
        arms.contains(
            "public static func inputN<Source: CodecEveryArmSaysSource>(_ source: Source) -> UInt8 {\n        switch source.body {\n        case .codecEveryArmSaysFar(let arm): return CodecEveryArmSaysFar.inputBare(arm)\n        case .codecEveryArmSaysNear(let arm): return CodecEveryArmSaysNear.inputN(arm)\n        }\n    }"
        ),
        "{arms}"
    );
    let carrier = swift_of("tests/forge/resources/codec_repeat_flag_carrier.scxml")
        .expect("a list whose entries are handed a bit of a carrier is written");
    assert!(
        carrier.contains("wide: UInt8(truncatingIfNeeded: (head >> 0) & 0x1)"),
        "a decode hands the bit of the carrier it has read: {carrier}"
    );
    // The entries say `wide` by the arm of their gate group (docs/adr/0065 §6 R4b), and the list
    // hands every one of them `head.W`, so the list is held to the uniform law of `wide` (§6 R8):
    // its type holds every entry to the start it holds, the printed encode hands the entries
    // nothing, and `W` is derived from the start.
    for (expected, why) in [
        (
            "    public enum EntriesWideSaid<Entry: CodecChainFlagEntrySource>: SceSaid {\n        public static func says(_ entry: Entry) -> Bool? {\n            CodecChainFlagEntry.inputWide(entry) != 0\n        }\n    }\n",
            "what an entry says of `wide` is read through its codec's reader",
        ),
        (
            "    public typealias EntriesLaw<Entry: CodecChainFlagEntrySource> = SceUniform<EntriesWideSaid<Entry>>\n",
            "the law is the uniform automaton over it",
        ),
        (
            "    var entries: Lawful<BoundedList<Entries_Entry, CodecRepeatFlagCarrier.EntriesBound>, CodecRepeatFlagCarrier.EntriesLaw<Entries_Entry>> { get }\n",
            "the protocol presents the list held to its law",
        ),
        (
            "head_folded = (head_folded & ~0x1) | ((UInt8(truncatingIfNeeded: (source.entries.start ? 1 : 0)) & 0x1) << 0)",
            "the bound flag is the list's start",
        ),
        (
            "let entries = try Lawful<BoundedList<CodecChainFlagEntry, CodecRepeatFlagCarrier.EntriesBound>, CodecRepeatFlagCarrier.EntriesLaw<CodecChainFlagEntry>>(decoding: try BoundedList<CodecChainFlagEntry, CodecRepeatFlagCarrier.EntriesBound>(decoding: entries_items), start: ((head >> 0) & 1) != 0)",
            "a decode holds the entries it read to the law from the bit the frame gave",
        ),
        (
            " = .init(defaultOf: .fewest(of: CodecChainFlagEntry()))\n",
            "a value that has read no frame holds the fewest entries from the start they say",
        ),
    ] {
        assert!(carrier.contains(expected), "{why}: {expected} in {carrier}");
    }
    assert!(
        !carrier.contains("let handed: Bool"),
        "a list held to its law is held where it is made, not by the encode: {carrier}"
    );
}

#[test]
fn a_list_held_to_a_flag_another_declaration_derives_is_checked_entry_by_entry() {
    // `head.W` is the presence of `extra`, so the flag is derived from it and not from the list's
    // entries, which say `wide`: the list has no law of its own, and the encode holds each entry
    // to the bit the carrier is written with (docs/adr/0065 §6 R4a, a check at the list).
    let code = swift_of_beside(
        "codec_list_checked",
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="codec_list_checked">
  <sce:import src="codec_chain_flag_entry.scxml" kind="codec" as="codec_chain_flag_entry">
    <sce:flag-bind input="wide" source="head.W"/>
  </sce:import>
  <datamodel>
    <sce:flags id="head" sce:type="uint8" sce:byte="0" sce:bit-size="8">
      <sce:flag name="W" bit="0"/>
    </sce:flags>
    <sce:field id="extra" sce:type="uint8" sce:byte="1" sce:bit-size="8"
               sce:present-if="head.W"/>
    <sce:field id="count" sce:type="uint8" sce:byte="2" sce:bit-size="8"/>
    <sce:repeat id="entries" type="codec_chain_flag_entry" sce:byte="3"
                count="count" max-count="4"/>
  </datamodel>
</scxml>"#,
        &["codec_chain_flag_entry.scxml"],
    )
    .expect("a list held to a derived flag is written");
    for (expected, why) in [
        (
            "    var entries: BoundedList<Entries_Entry, CodecListChecked.EntriesBound> { get }\n",
            "the list is no list held to a law",
        ),
        (
            "        do {\n            for entry in source.entries {\n                let handed: Bool = ((head_folded >> 0) & 1) != 0\n",
            "each entry is held to the bit the carrier is written with",
        ),
        (
            "if ((CodecChainFlagEntry.inputWide(entry) != 0) != handed) { throw CodecError.presentIfMismatch }",
            "an entry that says otherwise has no frame",
        ),
    ] {
        assert!(code.contains(expected), "{why}: {expected} in {code}");
    }
}

#[test]
fn a_gated_byte_string_is_checked_where_it_is_given() {
    let code = swift_of("tests/forge/resources/codec_present_if_tail.scxml")
        .expect("a gated tail is written");
    assert!(
        code.contains("if let value = self.payload, value.count > "),
        "an encode checks a gated field only when it is given: {code}"
    );
    assert!(
        code.contains("            if reader.remaining > "),
        "a decode checks it inside the gate: {code}"
    );
}

#[test]
fn what_a_variable_length_field_cannot_be_is_refused_by_name() {
    for (field, what) in [
        (
            r#"<sce:field id="a" sce:type="uint8" sce:byte="0" sce:bit-offset="4" sce:bit-size="4"/>
               <sce:field id="body" sce:type="bytes" sce:byte="1" sce:bit-size="tail" sce:max-size="8"/>"#,
            "a bit offset in a frame whose layout depends on what it holds",
        ),
        (
            r#"<sce:field id="n" sce:type="int8" sce:byte="0" sce:bit-size="8"/>
               <sce:field id="body" sce:type="bytes" sce:byte="1" sce:bit-size="length-ref" sce:length-field="n" sce:max-size="8"/>"#,
            "which is not an unsigned integer",
        ),
    ] {
        let refusal = swift_of_datamodel(field).expect_err(&format!("{field} is not lowered"));
        assert!(
            refusal.contains("has no Swift lowering yet") && refusal.contains(what),
            "{field}: {refusal}"
        );
    }
}

#[test]
fn a_length_held_in_the_bits_of_a_flag_is_read_shifted_down_and_masked() {
    let code = swift_of("tests/forge/resources/codec_length_ref_dotted_basic.scxml")
        .expect("a length in a flag is written");
    assert!(
        !code.contains("if ((carrier >> 4) & 0xF)"),
        "four bits cannot say more than the 15 bytes the document admits, so nothing is judged: {code}"
    );
    assert!(
        code.contains("public static let most: Int? = 15"),
        "the bytes are held to the bound by their type: {code}"
    );
    assert!(
        code.contains("Array(try reader.readSlice(Int(clamping: ((carrier >> 4) & 0xF))))"),
        "{code}"
    );
    // The length is derived into its four bits (docs/adr/0081 §3), the rest of the carrier as the
    // value holds it.
    assert!(
        code.contains(
            "carrier_folded = (carrier_folded & ~0xF0) | ((UInt8(truncatingIfNeeded: source.payload.byteCount) & 0xF) << 4)"
        ),
        "{code}"
    );
}

#[test]
fn a_codec_with_no_field_is_a_frame_of_no_bytes() {
    let code = swift_of("tests/forge/resources/codec_zenoh_keep_alive.scxml")
        .expect("a codec with no field is written");
    assert!(code.contains("public static let frameLength = 0"), "{code}");
    assert_eq!(
        code.matches("public init(").count(),
        1,
        "a second initializer of no parameter would declare the first again: {code}"
    );
    assert!(
        code.contains("let frame = [UInt8](repeating: 0, count: Self.frameLength)"),
        "a frame nothing is stored into is a constant: {code}"
    );
}

#[test]
fn a_field_of_bits_inside_a_byte_is_cut_and_shifted_into_it() {
    let code =
        swift_of("tests/forge/resources/codec_subbyte.scxml").expect("bit fields are written");
    assert!(
        code.contains("priority: try (cursor.peekInteger(UInt8.self, at: 0, .big) >> 5) & 0x7,"),
        "a decode shifts the bits down and masks them: {code}"
    );
    assert!(
        code.contains("direction: try (cursor.peekInteger(UInt8.self, at: 0, .big) >> 0) & 0x3"),
        "{code}"
    );
    assert!(
        code.contains("frame[0] |= (source.channel & 0x7) << 2"),
        "an encode cuts the value to its width and ors it into the byte: {code}"
    );
    assert!(
        code.contains("public static let frameLength = 1"),
        "three fields of one byte are one byte: {code}"
    );
}

#[test]
fn a_field_whose_bits_leave_its_byte_is_read_from_its_word_and_written_by_its_pieces() {
    let code = swift_of("tests/forge/resources/codec_optional_field_attributes.scxml")
        .expect("a field whose bits span two bytes is written");
    assert!(
        code.contains("reading: try (cursor.peekInteger(UInt16.self, at: 1, .big) >> 4) & 0xFFF"),
        "a decode reads the big-endian word of bytes 1 and 2, moves it down and masks it: {code}"
    );
    assert!(
        code.contains("frame[1] |= UInt8(truncatingIfNeeded: (source.reading >> 4) & 0xFF) << 0"),
        "byte 1 holds the high eight of the twelve bits: {code}"
    );
    assert!(
        code.contains("frame[2] |= UInt8(truncatingIfNeeded: source.reading & 0xF) << 4"),
        "the high nibble of byte 2 holds the low four: {code}"
    );
    assert!(
        code.contains("storeInteger(status_folded, into: &frame, at: 0, .little)"),
        "a field that is the whole of its integer is stored whole: {code}"
    );
    assert!(
        code.contains("public static let frameLength = 3"),
        "the frame runs to the end of the word: {code}"
    );
}

#[test]
fn a_field_that_is_not_the_whole_of_its_integer_is_converted_to_its_type() {
    let code = swift_of_datamodel(
        r#"<sce:field id="past" sce:type="uint8" sce:byte="0" sce:bit-offset="6" sce:bit-size="4"/>
    <sce:field id="wide" sce:type="uint16" sce:byte="2" sce:bit-offset="0" sce:bit-size="4"/>
    <sce:field id="three" sce:type="uint32" sce:byte="3" sce:bit-offset="4" sce:bit-size="20"/>
    <sce:field id="low" sce:type="uint16" sce:byte="6" sce:bit-size="8"/>"#,
    )
    .expect("bits of a word are written");
    for (expected, why) in [
        (
            "past: try UInt8(truncatingIfNeeded: (cursor.peekInteger(UInt16.self, at: 0, .big) >> 6) & 0xF)",
            "a word wider than the field's type is narrowed to it",
        ),
        (
            "frame[0] |= ((source.past >> 2) & 0x3) << 0",
            "the high byte of a big-endian word holds the high bits",
        ),
        (
            "frame[1] |= (source.past & 0x3) << 6",
            "and the low byte the low bits",
        ),
        (
            "wide: try UInt16(truncatingIfNeeded: (cursor.peekInteger(UInt8.self, at: 2, .big) >> 0) & 0xF)",
            "a word narrower than the field's type is widened to it",
        ),
        (
            "frame[2] |= UInt8(truncatingIfNeeded: source.wide & 0xF) << 0",
            "a piece of a field wider than a byte is narrowed to one",
        ),
        (
            "three: try (cursor.peekInteger(UInt32.self, bytes: 3, at: 3, .big) >> 4) & 0xFFFFF",
            "a word of three bytes is read by the count of its bytes",
        ),
        (
            "frame[3] |= UInt8(truncatingIfNeeded: (source.three >> 12) & 0xFF) << 0",
            "each byte of the word holds its piece",
        ),
        (
            "frame[5] |= UInt8(truncatingIfNeeded: source.three & 0xF) << 4",
            "the low bits of the word are in its last byte",
        ),
        (
            "low: try UInt16(truncatingIfNeeded: (cursor.peekInteger(UInt8.self, at: 6, .big) >> 0) & 0xFF)",
            "whole bytes fewer than the type has are a word of their own",
        ),
        (
            "frame[6] |= UInt8(truncatingIfNeeded: source.low & 0xFF) << 0",
            "and are written by their pieces, not as the type's bytes",
        ),
        (
            "public static let frameLength = 7",
            "the frame runs to the end of the last word",
        ),
    ] {
        assert!(code.contains(expected), "{why}: {expected} in {code}");
    }
}

#[test]
fn bits_a_swift_codec_cannot_lay_out_are_refused_by_name() {
    for (field, what) in [
        (
            r#"<sce:field id="small" sce:type="int16" sce:byte="0" sce:bit-size="8"/>"#,
            "8 bits where its type Int16 holds 16",
        ),
        (
            r#"<sce:field id="n" sce:type="uint16" sce:byte="0" sce:bit-size="12"/>
               <sce:field id="body" sce:type="bytes" sce:byte="2" sce:bit-size="tail" sce:max-size="8"/>"#,
            "12 bits that are not whole bytes, in a frame whose layout depends on what it holds",
        ),
    ] {
        let refusal = swift_of_datamodel(field).expect_err(&format!("{field} is not lowered"));
        assert!(
            refusal.contains("has no Swift lowering yet") && refusal.contains(what),
            "{field}: {refusal}"
        );
    }
}

#[test]
fn a_word_in_the_native_byte_order_is_read_and_written_as_a_big_endian_one() {
    let code = swift_of_datamodel(
        r#"<sce:field id="past" sce:type="uint16" sce:byte="0" sce:bit-offset="4" sce:bit-size="8" sce:endian="native"/>"#,
    )
    .expect("a word in the native byte order is written, as on every backend");
    for (expected, why) in [
        (
            "past: try (cursor.peekInteger(UInt16.self, at: 0, .big) >> 4) & 0xFF",
            "the decode reads the word as big-endian",
        ),
        (
            "frame[0] |= UInt8(truncatingIfNeeded: (source.past >> 4) & 0xF) << 0",
            "the first byte of the word is its most significant",
        ),
        (
            "frame[1] |= UInt8(truncatingIfNeeded: source.past & 0xF) << 4",
            "and the second its least",
        ),
    ] {
        assert!(code.contains(expected), "{why}: {expected} in {code}");
    }
}

#[test]
fn a_field_that_is_on_the_wire_on_a_flag_is_optional_and_read_only_when_the_flag_is_set() {
    let code = swift_of("tests/forge/resources/codec_present_if_basic.scxml")
        .expect("a gated field is written");
    assert!(code.contains("public var seq: UInt16? = nil"), "{code}");
    assert!(
        code.contains(
            "var seq: UInt16? = nil\n        if (flags & 0x1) != 0 {\n            seq = try reader.readInteger(UInt16.self, .big)\n        }"
        ),
        "a decode reads the field only when the bit of the carrier it has read is set: {code}"
    );
    assert!(
        code.contains(
            "if let held = source.seq {\n            appendInteger(held, to: &frame, .big)\n        }"
        ),
        "an encode writes the field where the value holds it: {code}"
    );
    assert!(
        code.contains("var seq: UInt16? { get }"),
        "a source presents the field as an optional: {code}"
    );
    // The flag that gates it is derived from its presence (docs/adr/0058 §1): the carrier is
    // written with the bit set exactly where the value holds the field, whatever the value's own
    // bit says.
    for (expected, why) in [
        (
            "var flags_folded = source.flags",
            "the carrier as the value presents it",
        ),
        (
            "flags_folded = (flags_folded & ~0x1) | ((source.seq != nil) ? 0x1 : 0)",
            "the flag is the field's presence",
        ),
        (
            "appendInteger(flags_folded, to: &frame, .big)",
            "the carrier is written with the flag folded in",
        ),
    ] {
        assert!(code.contains(expected), "{why}: {expected} in {code}");
    }
    assert!(
        code.contains("public static let frameLength = 1"),
        "a field that may be absent adds nothing to the fewest bytes: {code}"
    );
}

#[test]
fn a_gate_the_encode_cannot_derive_is_a_check_before_the_first_byte() {
    // `trailer` is on the wire where `has_seq || wants_trailer`: `has_seq` is derived from `seq`,
    // and the disjunction is no flag of the trailer's own, so the value is held to it (docs/adr/0065
    // §6 R4c, §2).
    let code = swift_of("tests/forge/resources/codec_present_if_checked.scxml")
        .expect("a codec whose encode refuses a value is written");
    let check = "if (((flags_folded >> 0) & 1) != 0 || ((flags_folded >> 1) & 1) != 0) != \
                 (source.trailer != nil) { throw CodecError.presentIfMismatch }";
    let check_at = code
        .find(check)
        .unwrap_or_else(|| panic!("the trailer is held to its gate: {check} in {code}"));
    let fold_at = code
        .find("flags_folded = (flags_folded & ~0x1) | ((source.seq != nil) ? 0x1 : 0)")
        .unwrap_or_else(|| panic!("has_seq is derived from seq: {code}"));
    let frame_at = code
        .find("var frame: [UInt8] = []")
        .unwrap_or_else(|| panic!("the frame grows field by field: {code}"));
    assert!(
        fold_at < check_at && check_at < frame_at,
        "the gate reads the derived flag, and the check comes before the first byte: {code}"
    );
    assert!(
        code.contains("A value the encoding cannot carry is refused before the first"),
        "the encode says that it refuses: {code}"
    );
}

#[test]
fn a_negated_clause_is_on_a_clear_bit_and_a_disjunction_on_any_clause() {
    let negated = swift_of("tests/forge/resources/codec_present_if_negation.scxml")
        .expect("a negated gate is written");
    assert!(negated.contains("(flags & 0x1) == 0"), "{negated}");
    let either = swift_of("tests/forge/resources/codec_present_if_disjunction.scxml")
        .expect("a disjunction is written");
    assert!(
        either.contains("(flags & 0x1) != 0 || (flags & 0x2) != 0"),
        "any clause of a disjunction puts the field on the wire: {either}"
    );
}

#[test]
fn a_gate_group_is_one_property_whose_arm_the_encode_derives_the_gates_from() {
    // docs/adr/0065 §6 R4c: `seq` is on the wire under `wants_a || wants_b`, two flags nothing else
    // reads, so the value holds the arm of the disjunction and the encode derives both flags.
    let either = swift_of("tests/forge/resources/codec_present_if_disjunction.scxml")
        .expect("a disjunction is written");
    for (expected, why) in [
        (
            "public enum CodecPresentIfDisjunctionFlagsWantsAOrWantsB: Equatable, Sendable {",
            "the group is an enum of its arms",
        ),
        (
            "    case wantsAWantsB(seq: UInt16)\n",
            "an arm holds each member by its name",
        ),
        (
            "    public var flags_wants_a_or_wants_b: CodecPresentIfDisjunctionFlagsWantsAOrWantsB? = nil\n",
            "a value that may hold no arm holds the group as an optional",
        ),
        (
            "var seq_member: UInt16? = nil",
            "the encode reads the member off the arm the value holds",
        ),
        (
            "    if let held = seq_member {\n",
            "the member is written where the arm holds it",
        ),
        (
            "flags_folded = (flags_folded & ~0x1) | ((source.flags_wants_a_or_wants_b.map { [",
            "the flag is derived from the arm",
        ),
        (
            ".contains($0.arm) } ?? false) ? 0x1 : 0)",
            "a value that holds no arm clears the flag",
        ),
        (
            "case .wantsAWantsB: return ",
            "the arm says its place among the arms",
        ),
    ] {
        assert!(either.contains(expected), "{why}: {expected} in {either}");
    }
    // docs/adr/0065 §6 R4b: `wide` decides `long_value` and `short_value` on its two values, so a
    // value always holds one of the two arms, says the input by which, and a decode makes the arm
    // the input it is handed names.
    let entry = swift_of("tests/forge/resources/codec_chain_flag_entry.scxml")
        .expect("a group of one input is written");
    for (expected, why) in [
        (
            "    case wide(long_value: UInt16)\n",
            "the arm while the input is set",
        ),
        (
            "    case notWide(short_value: UInt8)\n",
            "the arm while it is clear",
        ),
        (
            "    public var wide: CodecChainFlagEntryWide = .notWide(short_value: 0)\n",
            "a value always holds an arm, a frame of zeros the one of every gate clear",
        ),
        (
            "        [0].contains(source.wide.arm) ? 1 : 0\n",
            "the value says the input by the arm it holds",
        ),
        (
            "        if (wide & 0x01) != 0 {\n            wide_group = .wide(long_value: long_value!)\n        } else {\n            wide_group = .notWide(short_value: short_value!)\n        }\n",
            "a decode makes the arm the input names",
        ),
        (
            "public init(header: UInt8, wide: CodecChainFlagEntryWide) {",
            "the value is made of its elements and its group, in the order of the document",
        ),
    ] {
        assert!(entry.contains(expected), "{why}: {expected} in {entry}");
    }
}

/// The Swift source of the codec document `document`, named `name`, written beside a copy of each
/// resource codec `imports` names (a file under `tests/forge/resources`), which it imports by the
/// file's stem; or the refusal it is answered with.
fn swift_of_beside(name: &str, document: &str, imports: &[&str]) -> Result<String, String> {
    let dir = tempfile::tempdir().expect("tempdir");
    for import in imports {
        std::fs::copy(
            repo_root().join("tests/forge/resources").join(import),
            dir.path().join(import),
        )
        .expect("copy an imported codec");
    }
    let path = dir.path().join(format!("{name}.scxml"));
    std::fs::write(&path, document).expect("write the codec");
    let output = compile_forge_file(&path, Language::Swift, &[], &ForgeCompileOptions::default())
        .map_err(|e| format!("{e:?}"))?;
    let (_, code) = output
        .files
        .into_iter()
        .next()
        .expect("a document that is written writes a file");
    Ok(code)
}

#[test]
fn a_gate_group_that_holds_a_body_is_generic_over_the_source_of_it() {
    // docs/adr/0065 §6 R4c: `detail` is on the wire under `near || far`, two flags nothing else
    // reads, so the value holds the arm of the disjunction; the arm holds the body as a source of
    // its codec, which the protocol names by an associated type the value binds to the codec's own
    // value, so a host's type is written with no copy.
    let code = swift_of_beside(
        "codec_group_body",
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="codec_group_body">
  <sce:import src="codec_simple_frame.scxml" kind="codec" as="codec_simple_frame"/>
  <datamodel>
    <sce:flags id="flags" sce:type="uint8" sce:byte="0" sce:bit-size="8">
      <sce:flag name="near" bit="0"/>
      <sce:flag name="far" bit="1"/>
    </sce:flags>
    <sce:embed id="detail" type="codec_simple_frame" sce:byte="1"
               sce:present-if="flags.near || flags.far"/>
  </datamodel>
</scxml>"#,
        &["codec_simple_frame.scxml"],
    )
    .expect("a group that holds a body is written");
    for (expected, why) in [
        (
            "public enum CodecGroupBodyFlagsNearOrFar<Detail_Source: CodecSimpleFrameSource> {\n",
            "the group's type is generic over the body's source",
        ),
        (
            "    case nearFar(detail: Detail_Source)\n",
            "an arm holds the body as that source",
        ),
        (
            "extension CodecGroupBodyFlagsNearOrFar: Equatable where Detail_Source: Equatable {}\n",
            "the group compares where its sources do",
        ),
        (
            "extension CodecGroupBodyFlagsNearOrFar: Sendable where Detail_Source: Sendable {}\n",
            "the group is sent where its sources are",
        ),
        (
            "    associatedtype Detail_Source: CodecSimpleFrameSource\n",
            "the protocol names the body's source",
        ),
        (
            "    var flags_near_or_far: CodecGroupBodyFlagsNearOrFar<Detail_Source>? { get }\n",
            "the protocol requires the group over that source",
        ),
        (
            "    public typealias Detail_Source = CodecSimpleFrame\n",
            "the value binds the source to the body codec's own value",
        ),
        (
            "    public var flags_near_or_far: CodecGroupBodyFlagsNearOrFar<Detail_Source>? = nil\n",
            "the value holds the group, none of it until a frame is read",
        ),
        (
            "var detail_member: Source.Detail_Source? = nil",
            "the encode keeps the body as the source presents it",
        ),
        (
            "case .nearFar(let detail)?: detail_member = detail",
            "off the arm the value holds",
        ),
        (
            "        if let held = detail_member {\n            var piece = BytesSink()\n            try CodecSimpleFrame.encodeSource(held, into: &piece)\n            detail_bytes = piece.bytes\n        }\n",
            "the body is encoded once, by its own codec's encode",
        ),
        (
            "if let held = detail_bytes { frame += held }",
            "and written where the arm holds it",
        ),
        (
            "flags_near_or_far_group = .nearFar(detail: detail!)",
            "a decode makes the arm the flags name",
        ),
    ] {
        assert!(code.contains(expected), "{why}: {expected} in {code}");
    }
    assert!(
        !code.contains("var detail: Detail_Source? { get }")
            && !code.contains("public var detail: "),
        "the body is no element of its own: {code}"
    );
}

#[test]
fn a_gate_group_that_holds_a_list_and_a_stated_length_derives_the_length_on_one_side() {
    // docs/adr/0065 §6 R4b: `after_shm` decides `value` on its clear side and `encoding` and the
    // slices on its set side, where `value_len`, which measures `value`, is stated by the value.
    let code = swift_of("tests/forge/resources/codec_zenoh_value_zbuf.scxml")
        .expect("a group that holds a list is written");
    for (expected, why) in [
        (
            "public enum CodecZenohValueZbufAfterShm<Slices_Entry: CodecZenohValueSliceSource> {\n",
            "the group's type is generic over the source of the list's entries",
        ),
        (
            "    case afterShm(encoding: UInt64, slices: BoundedList<Slices_Entry, CodecZenohValueZbuf.SlicesBound>, value_len: UInt64)\n",
            "the arm holds the list within its count bound, and the length it states",
        ),
        (
            "    public var after_shm: CodecZenohValueZbufAfterShm<Slices_Entry> = .notAfterShm(value: .fewest)\n",
            "a frame of zeros holds the arm of the input clear",
        ),
        (
            "var slices_member: BoundedList<Source.Slices_Entry, CodecZenohValueZbuf.SlicesBound>? = nil",
            "the encode keeps the list as the source presents it",
        ),
        (
            "let value_len_derived: UInt64 = value_member != nil ? UInt64((value_member?.byteCount ?? 0)) : (value_len_member ?? 0)",
            "the length is the bytes' where the arm holds them, else what the arm states",
        ),
        (
            "let slice_count_derived: UInt64 = slices_member != nil ? UInt64((slices_member?.count ?? 0)) : 0",
            "the count is the entries the arm holds",
        ),
        (
            "        if let held = slices_member {\n            var slices_entries = BytesSink()\n            for entry in held {\n                try CodecZenohValueSlice.encodeSource(entry, into: &slices_entries)\n            }\n            frame += slices_entries.bytes\n        }\n",
            "the list is walked where the arm holds it",
        ),
        (
            "after_shm_group = .afterShm(encoding: encoding!, slices: slices!, value_len: value_len)",
            "a decode makes the arm of the stated length, which it reads always",
        ),
        (
            "slices = try BoundedList<CodecZenohValueSlice, CodecZenohValueZbuf.SlicesBound>(decoding: slices_items)",
            "a decode holds the list to its bound",
        ),
    ] {
        assert!(code.contains(expected), "{why}: {expected} in {code}");
    }
}

#[test]
fn a_list_held_to_its_law_in_the_arm_of_a_group_derives_the_flag_from_its_start_there() {
    // docs/adr/0065 §6 R4b, R8: `head.L` decides `level` and the list on its set side and `note`
    // on its clear side; the list's entries say `wide`, which the list hands them from `head.W`, so
    // the arm holds the list held to its law, and `W` is its start where the arm holds it and the
    // value's own where it does not.
    let code = swift_of("tests/forge/resources/codec_repeat_flag_group.scxml")
        .expect("a group that holds a list held to its law is written");
    for (expected, why) in [
        (
            "    case l(level: UInt8, entries: Lawful<BoundedList<Entries_Entry, CodecRepeatFlagGroup.EntriesBound>, CodecRepeatFlagGroup.EntriesLaw<Entries_Entry>>)\n",
            "the arm holds the list held to its law, over the group's source of the entries",
        ),
        (
            "var entries_member: Lawful<BoundedList<Source.Entries_Entry, CodecRepeatFlagGroup.EntriesBound>, CodecRepeatFlagGroup.EntriesLaw<Source.Entries_Entry>>? = nil",
            "the encode keeps the list as the source presents it",
        ),
        (
            "if entries_member != nil { head_folded = (head_folded & ~0x1) | ((UInt8(truncatingIfNeeded: (entries_member.map { $0.start ? 1 : 0 } ?? 0)) & 0x1) << 0) }",
            "the flag is the start where the arm holds the list, else the value's own",
        ),
        (
            "let count_derived: UInt8 = entries_member != nil ? UInt8((entries_member?.count ?? 0)) : 0",
            "the count is the entries the arm holds",
        ),
        (
            "head_l_group = .l(level: level!, entries: entries!)",
            "a decode makes the arm of the list it held to its law",
        ),
    ] {
        assert!(code.contains(expected), "{why}: {expected} in {code}");
    }
}

#[test]
fn an_input_a_list_held_to_its_law_says_is_read_off_its_start() {
    // docs/adr/0065 §6 R8: the codec hands its own `wide` on to every entry of its gated chain,
    // whose entries say it, so the value says `wide` by the chain's start, and nothing where the
    // chain is absent (§6 R4e); the decode holds the chain from the input it is handed.
    let code = swift_of("tests/forge/resources/codec_chain_flag_forward.scxml")
        .expect("a chain that says its holder's input is written");
    for (expected, why) in [
        (
            "public static func inputWide<Source: CodecChainFlagForwardSource>(_ source: Source) -> UInt8? {\n        source.entries.map { $0.start ? 1 : 0 }\n    }",
            "the input is the start of the chain, none while the chain is absent",
        ),
        (
            "(decoding: entries_items), start: (wide & 0x01) != 0)",
            "a decode holds the chain from the input it is handed",
        ),
        (
            "public static func encodeSource<Source: CodecChainFlagForwardSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {",
            "the encode is not handed the input the value says",
        ),
    ] {
        assert!(code.contains(expected), "{why}: {expected} in {code}");
    }
}

#[test]
fn a_gated_variable_length_field_is_an_optional_that_reads_and_writes_as_an_ungated_one() {
    let tail = swift_of("tests/forge/resources/codec_present_if_tail.scxml")
        .expect("a gated tail is written");
    assert!(
        tail.contains(": BoundedBytes<CodecPresentIfTail.") && tail.contains("Bound>? = nil"),
        "{tail}"
    );
    assert!(
        tail.contains("if let held = source.") && tail.contains("    held.append(to: &frame)"),
        "a printed encode appends the bytes where the value holds them: {tail}"
    );
    assert!(
        tail.contains("Array(try reader.readSlice(reader.remaining))"),
        "{tail}"
    );
    let vle = swift_of("tests/forge/resources/codec_present_if_vle.scxml")
        .expect("a gated variable-length integer is written");
    assert!(vle.contains("? = nil"), "{vle}");
    assert!(vle.contains("try reader.readVle("), "{vle}");
}

#[test]
fn a_gate_on_the_entries_of_a_chain_reads_a_boolean_bound_where_the_chain_is_read() {
    let code = swift_of("tests/forge/resources/codec_chain_has_tagged.scxml")
        .expect("a gate on the entries of a chain is written");
    assert!(
        code.contains("let _has_entries_7 = entries.contains(where: { _e in chainIdentifier(_e.entry_type) == 7 })"),
        "the chain is asked for the identifier it holds, once it is read: {code}"
    );
    assert!(
        code.contains("if _has_entries_7 || (header & 0x1) != 0 {"),
        "a clause on a chain joins a clause on a flag: {code}"
    );
    assert!(
        code.contains("if !_has_entries_9 {"),
        "a negated clause is on the wire when no entry has the identifier: {code}"
    );
}

#[test]
fn an_encode_asks_the_value_what_the_decode_asked_the_frame() {
    let code = swift_of("tests/forge/resources/codec_chain_has_tagged.scxml")
        .expect("a gate on the entries of a chain is written");
    // The encode is given the entries and the field the gate holds, and refuses a value where the two
    // disagree, so the chain is asked here as it was asked of the frame.
    let encode = code
        .split("public func encode")
        .nth(1)
        .expect("the codec has an encode");
    assert!(
        encode.contains("let _has_entries_7 = self.entries.contains(where:"),
        "{code}"
    );
}

#[test]
fn a_chain_hands_its_entries_an_input_read_off_the_entry_before() {
    let code = swift_of("tests/forge/resources/codec_chain_prev_carrier.scxml")
        .expect("a chain that supplies an input to its entries is written");
    assert!(
        code.contains("var _prev_entries_after_marker: UInt8 = 0\n"),
        "the first entry has none before it: {code}"
    );
    assert!(
        code.contains(
            "CodecChainPrevEntry.decode(&reader, after_marker: _prev_entries_after_marker, tagged: UInt8(truncatingIfNeeded: (head >> 0) & 0x1))"
        ),
        "the chain's input and the carrier's bit are passed in the entry's declared order: {code}"
    );
    assert!(
        code.contains("_prev_entries_after_marker = (chainIdentifier(entry.extId) == 4) ? 1 : 0"),
        "it is rewritten from the entry just handled: {code}"
    );
    // Each entry says `after_marker` by the arm of its gate group (docs/adr/0065 §6 R4b), and
    // `tagged` by its one field gated on it, so the chain is held to two laws run together
    // (§6 R8): `tagged` the start the value gives, which `head.T` is derived from, and
    // `after_marker` whether the entry before is of the kind 4, the first after none.
    for (expected, why) in [
        (
            "    public enum EntriesAfterMarkerKind<Entry: CodecChainPrevEntrySource>: SceKind {\n        public static func matches(_ entry: Entry) -> Bool {\n            ((chainIdentifier(entry.header) >> 0) & 0xF) == 0x4\n        }\n    }\n",
            "the kind is read off the entry's identifier as the chain reads it",
        ),
        (
            "    public typealias EntriesLaw<Entry: CodecChainPrevEntrySource> = SceLaws<SceUniform<EntriesTaggedSaid<Entry>>, SceAfterKey<EntriesAfterMarkerSaid<Entry>, EntriesAfterMarkerKind<Entry>>>\n",
            "the two laws run together in the order of the inputs",
        ),
        (
            "head_folded = (head_folded & ~0x1) | ((UInt8(truncatingIfNeeded: (source.entries.start.first ? 1 : 0)) & 0x1) << 0)",
            "the bound flag is the start of the first law",
        ),
        (
            "(decoding: entries_items), start: SceStarts(((head >> 0) & 1) != 0, SceFixedStart()))",
            "a decode holds the chain from the bit the frame gave and the fixed start",
        ),
    ] {
        assert!(code.contains(expected), "{why}: {expected} in {code}");
    }
    let encode = code
        .split("public static func encodeSource")
        .nth(1)
        .expect("the codec's encode is printed");
    assert!(
        !encode.contains("entries_before") && !encode.contains("after_marker:"),
        "the encode hands the entries nothing and checks none of them: {code}"
    );
}

#[test]
fn a_chain_to_the_end_of_the_frame_is_read_while_bytes_are_left_to_its_depth() {
    let code = swift_of("tests/forge/resources/codec_tlv_chain_basic.scxml")
        .expect("a chain to the end of the frame is written");
    // docs/adr/0065 §6 R2: the chain is held in a list type bounded by its depth, which holds no
    // entry until a frame is read.
    assert!(
        code.contains(
            "\n    public var extensions: BoundedList<CodecTlvEntry, CodecTlvChainBasic.ExtensionsBound> = .fewest(of: CodecTlvEntry())\n"
        ),
        "{code}"
    );
    assert!(
        code.contains("public enum ExtensionsBound: SceCountBound {\n        public static let least = 0\n        public static let most: Int? = 8\n"),
        "the depth is the chain's count bound: {code}"
    );
    assert!(
        code.contains("for _ in 0..<8 {\n            if reader.remaining == 0 { break }\n            let entry = try CodecTlvEntry.decode(&reader)\n            extensions_items.append(entry)\n        }\n        if reader.remaining > 0 {\n            throw CodecError.tooManyElements\n        }"),
        "an entry past the depth is refused and not left on the wire unread: {code}"
    );
    assert!(
        code.contains(
            "let extensions = try BoundedList<CodecTlvEntry, CodecTlvChainBasic.ExtensionsBound>(decoding: extensions_items)"
        ),
        "a decode makes the chain through its bound's door: {code}"
    );
    // The encode holds no check of the depth: a chain past it is no value of its type.
    assert!(
        !code.contains("extensions.count > 8"),
        "an encode does not count a chain its type bounds: {code}"
    );
    assert!(
        code.contains(
            "        var extensions_entries = BytesSink()\n        for entry in source.extensions {\n            try CodecTlvEntry.encodeSource(entry, into: &extensions_entries)\n        }\n        frame += extensions_entries.bytes\n"
        ),
        "an encode writes each entry in turn by its codec's encode: {code}"
    );
}

#[test]
fn a_chain_closed_by_an_entry_flag_is_read_until_an_entry_clears_it() {
    let code = swift_of("tests/forge/resources/codec_zenoh_err.scxml")
        .expect("a chain closed by an entry flag is written");
    // The codec is printed from its plan: its chain is a bounded list of entries, optional where
    // its gate leaves it out (docs/adr/0065 §6 R2).
    assert!(
        code.contains(
            "public var extensions: BoundedList<CodecZenohExtEntry, CodecZenohErr.ExtensionsBound>? = nil"
        ),
        "a gated chain is optional: {code}"
    );
    // The protocol states no depth for the extensions, so the walk is not counted: it ends on the
    // entry that clears the flag, and an entry that promises another the frame lacks is a read the
    // frame is too short for. With no depth there is no entry one past it to refuse.
    assert!(
        code.contains("while true {\n                let entry = try CodecZenohExtEntry.decode(&reader)\n                extensions_more = entry.Z\n                extensions_items.append(entry)\n                if !extensions_more { break }"),
        "{code}"
    );
    assert!(
        !code.contains("0..<none") && !code.contains("if extensions_more {"),
        "a chain with no depth states no bound to count to or to refuse past: {code}"
    );
    // docs/adr/0065 §6 R7: a chain closed by a flag holds at least one entry, which its count bound
    // states and its type holds, so the encode counts nothing; the protocol states no depth.
    assert!(
        code.contains("public enum ExtensionsBound: SceCountBound {\n        public static let least = 1\n        public static let most: Int? = nil\n"),
        "a chain closed by a flag holds at least one entry: {code}"
    );
    assert!(
        !code.contains("value.count < 1"),
        "an encode does not count a chain its type bounds: {code}"
    );
    // The walk sets the flag on every entry but the last, whatever the entry presents.
    assert!(
        code.contains("var extensions_left = held.count")
            && code.contains(
                "try CodecZenohExtEntry.encodeSource(entry, into: &extensions_entries, overrides: CodecZenohExtEntry.Overrides(headerZ: extensions_left != 0))"
            ),
        "{code}"
    );
}

#[test]
fn two_lists_of_one_count_are_one_value_in_the_arm_that_holds_both() {
    // docs/adr/0065 §6 R5: `links` is always there and `weights` where `options.H` is set, both
    // of the count `links_len`; the arm of `H` set holds the two as one `Aligned`, which the
    // runtime makes only of two lists of one count, and the count is derived from the first.
    let code = swift_of("tests/forge/resources/codec_repeat_unconditional_count.scxml")
        .expect("two lists of one count are written");
    for (expected, why) in [
        (
            "public enum CodecRepeatUnconditionalCountOptionsH<Links_Entry: CodecRepeatElemSource, Weights_Entry: CodecRepeatElemSource> {\n",
            "the group is generic over the source of each list's entries",
        ),
        (
            "    case h(links_weights: Aligned<BoundedList<Links_Entry, CodecRepeatUnconditionalCount.LinksBound>, BoundedList<Weights_Entry, CodecRepeatUnconditionalCount.WeightsBound>>)\n",
            "the arm that holds both holds them as one value of one count",
        ),
        (
            "    case notH(links: BoundedList<Links_Entry, CodecRepeatUnconditionalCount.LinksBound>)\n",
            "the other arm holds the first alone",
        ),
        (
            "case .h(let links_weights): links_member = links_weights.first; weights_member = links_weights.second",
            "the encode reads each list out of the pair",
        ),
        (
            "UInt8((links_member?.count ?? 0))",
            "the count is the first list's, which every arm holds",
        ),
        (
            "for _ in 0..<Int(clamping: links_len) {\n            links_items.append(try CodecRepeatElem.decode(&reader))\n        }\n        let links = try BoundedList<CodecRepeatElem, CodecRepeatUnconditionalCount.LinksBound>(decoding: links_items)",
            "a decode reads as many entries as the count says, into a local set when whole",
        ),
        (
            "options_h_group = .h(links_weights: try Aligned(decoding: links, weights!))",
            "a decode pairs the two lists it read by the one count",
        ),
        (
            "    public var options_h: CodecRepeatUnconditionalCountOptionsH<Links_Entry, Weights_Entry> = .notH(links: .fewest(of: CodecRepeatElem()))\n",
            "a frame of zeros holds the first list alone",
        ),
        (
            "public static let frameLength = 2",
            "a list adds nothing to the fewest bytes",
        ),
    ] {
        assert!(code.contains(expected), "{why}: {expected} in {code}");
    }
}

#[test]
fn a_list_to_the_end_of_the_frame_is_read_while_bytes_are_left() {
    let code = swift_of("tests/forge/resources/codec_until_eof_basic.scxml")
        .expect("a list to the end is written");
    assert!(
        code.contains("while reader.remaining > 0 {\n            if msgs_items.count > 63 {\n                throw CodecError.tooManyElements\n            }\n            msgs_items.append(try CodecRepeatElem.decode(&reader))"),
        "an entry past the bound is refused before it is read: {code}"
    );
}

#[test]
fn a_list_is_held_to_its_declared_count_on_decode_and_on_encode() {
    let code = swift_of("tests/forge/resources/codec_repeat_present_if_basic.scxml")
        .expect("a gated list is written");
    assert!(
        code.contains("if (num_elems ?? 0) > 32 {\n                throw CodecError.tooManyElements\n            }\n            var elems_items"),
        "the count is judged before an entry is read, from the count a gate may leave absent: {code}"
    );
    // docs/adr/0081 §3: the count is derived from the list where the value holds one, and the
    // value holds no count; a list past the bound is no value of its type (docs/adr/0065 §6 R2).
    assert!(
        code.contains("let num_elems_derived: UInt8 = source.elems != nil ? UInt8((source.elems?.count ?? 0)) : 0"),
        "the count is the entries the list holds: {code}"
    );
    assert!(
        !code.contains("value.count > 32")
            && !code.contains("public var num_elems")
            && !code.contains("num_elems: num_elems"),
        "an encode neither counts a list its type bounds nor reads a stated count: {code}"
    );
    assert!(
        code.contains("        if let held = source.elems {\n            var elems_entries = BytesSink()\n            for entry in held {\n                try CodecRepeatElem.encodeSource(entry, into: &elems_entries)\n            }\n            frame += elems_entries.bytes\n        }\n"),
        "a gated list is written where the value holds it: {code}"
    );
}

#[test]
fn a_printed_list_presents_its_entries_as_sources_of_their_codec_within_a_count_bound() {
    let code = swift_of("tests/forge/resources/codec_until_eof_basic.scxml")
        .expect("a list to the end of the frame is written");
    assert!(
        code.contains("    associatedtype Msgs_Entry: CodecRepeatElemSource\n")
            && code.contains(
                "    var msgs: BoundedList<Msgs_Entry, CodecUntilEofBasic.MsgsBound> { get }\n"
            )
            && code.contains("    public typealias Msgs_Entry = CodecRepeatElem\n"),
        "each entry is presented as a source of its codec, bound to its value: {code}"
    );
    assert!(
        code.contains("public enum MsgsBound: SceCountBound {\n        public static let least = 0\n        public static let most: Int? = 64\n"),
        "the list's max-count is its count bound: {code}"
    );
    assert!(
        code.contains("let msgs = try BoundedList<CodecRepeatElem, CodecUntilEofBasic.MsgsBound>(decoding: msgs_items)"),
        "a decode makes the list through its bound's door: {code}"
    );
}

#[test]
fn a_count_of_entries_is_derived_from_the_list_and_is_no_member_of_the_value() {
    let code = swift_of("tests/forge/resources/codec_repeat_basic.scxml")
        .expect("a counted list is written");
    let counted = code
        .find("        let num_frags_derived: UInt8 = UInt8(source.frags.count)\n")
        .unwrap_or_else(|| panic!("the count is the entries the list holds: {code}"));
    let written = code
        .find("        var frags_entries = BytesSink()\n")
        .unwrap_or_else(|| panic!("the entries are written after the count: {code}"));
    assert!(counted < written, "{code}");
    assert!(
        code.contains(
            "public init(frags: BoundedList<CodecRepeatElem, CodecRepeatBasic.FragsBound>)"
        ) && !code.contains("public var num_frags"),
        "the value holds the list and not its count: {code}"
    );
}

#[test]
fn a_printed_chain_hands_each_entry_what_the_entry_before_supplies() {
    let code = swift_of("tests/forge/resources/codec_chain_handed.scxml")
        .expect("a chain that supplies an input to its entries is written");
    // The entry does not say `after_marker`, so the walk hands it: clear for the first entry, then
    // set where the entry just written is of the kind 4 (`entry-flag-bind previous-entry-id`).
    assert!(
        code.contains(
            "        var entries_after_marker_previous: UInt8 = 0\n        var entries_left = source.entries.count\n        for entry in source.entries {\n            entries_left -= 1\n            try CodecChainHandedEntry.encodeSource(entry, into: &entries_entries, after_marker: entries_after_marker_previous, overrides: CodecChainHandedEntry.Overrides(headerZ: entries_left != 0))\n            entries_after_marker_previous = ((chainIdentifier(entry.header) >> 0) & 0xF) == 0x4 ? 1 : 0\n        }\n"
        ),
        "{code}"
    );
    assert!(
        code.contains("public static func encodeSource<Source: CodecChainHandedSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {"),
        "the chain takes no input of its own and passes on its entries' refusal: {code}"
    );
}

#[test]
fn a_chain_sets_the_flag_its_entries_end_on_through_each_entrys_overrides() {
    let code = swift_of("tests/forge/resources/codec_chain_depth_plain.scxml")
        .expect("a chain closed by an entry flag is written");
    // docs/adr/0065 §6 R7: the flag is set on every entry but the last, whatever the entry
    // presents, and the chain holds one entry or more.
    assert!(
        code.contains(
            "        var extensions_left = source.extensions.count\n        for entry in source.extensions {\n            extensions_left -= 1\n            try CodecChainDepthEntry.encodeSource(entry, into: &extensions_entries, overrides: CodecChainDepthEntry.Overrides(headerZ: extensions_left != 0))\n        }\n"
        ),
        "{code}"
    );
    assert!(
        code.contains("public enum ExtensionsBound: SceCountBound {\n        public static let least = 1\n        public static let most: Int? = 8\n"),
        "a chain closed by its entries' flag holds at least one entry: {code}"
    );
    assert!(
        code.contains(" = .fewest(of: CodecChainDepthEntry())"),
        "a value that has read no frame holds the fewest entries its bound admits: {code}"
    );
}

#[test]
fn an_embedded_codec_swift_does_not_write_is_refused_by_name() {
    // An embedded codec that has no Swift lowering (here one with a DMA alignment) leaves the
    // codec that embeds it naming a type that is not there.
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::copy(
        repo_root().join("tests/forge/resources/codec_dma_aligned_basic.scxml"),
        dir.path().join("codec_dma_aligned_basic.scxml"),
    )
    .expect("copy the aligned codec");
    let document = dir.path().join("codec_embed_aligned.scxml");
    std::fs::write(
        &document,
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="codec_embed_aligned">
  <sce:import src="codec_dma_aligned_basic.scxml" kind="codec" as="codec_dma_aligned_basic"/>
  <datamodel>
    <sce:field id="tag" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:embed id="inner" type="codec_dma_aligned_basic" sce:byte="1"/>
  </datamodel>
</scxml>"#,
    )
    .expect("write the embedding codec");
    let Err(refusal) = compile_forge_file(
        &document,
        Language::Swift,
        &[],
        &ForgeCompileOptions::default(),
    ) else {
        panic!("an embedded codec Swift does not write is not written");
    };
    assert!(
        format!("{refusal:?}").contains("which has no Swift lowering"),
        "{refusal:?}"
    );
}

#[test]
fn an_embedded_codec_without_a_length_is_read_from_the_frames_own_cursor() {
    let code = swift_of("tests/forge/resources/codec_embed_basic.scxml")
        .expect("an embedded codec is written");
    assert!(
        code.contains("public var locator: CodecZenohLocator = CodecZenohLocator()"),
        "{code}"
    );
    assert!(
        code.contains("let locator = try CodecZenohLocator.decode(&reader)"),
        "{code}"
    );
}

#[test]
fn an_embedded_codec_is_laid_in_place_and_adds_its_own_length() {
    let code = swift_of("tests/forge/resources/codec_zenoh_timestamp_ext.scxml")
        .expect("a length-bounded embedded codec is written");
    assert!(
        code.contains("public var ts: CodecZenohTimestamp = CodecZenohTimestamp()"),
        "{code}"
    );
    // docs/adr/0081 §3: the body is encoded into bytes of its own once, before the frame, and its
    // encoded length, which `ext_size` holds, is the count of them: the value holds no `ext_size`.
    let encoded = code
        .find(
            "        var ts_piece = BytesSink()\n        try CodecZenohTimestamp.encodeSource(source.ts, into: &ts_piece)\n        let ts_bytes = ts_piece.bytes\n",
        )
        .unwrap_or_else(|| panic!("the body is encoded by its own codec: {code}"));
    let counted = code
        .find("        let ext_size_derived: UInt64 = UInt64(ts_bytes.count)\n")
        .unwrap_or_else(|| panic!("the length is the count of the body's bytes: {code}"));
    let written = code
        .find("        frame += ts_bytes\n")
        .unwrap_or_else(|| panic!("an encode writes the bytes where the field stands: {code}"));
    assert!(encoded < counted && counted < written, "{code}");
    assert!(
        code.contains("    associatedtype Ts_Source: CodecZenohTimestampSource\n")
            && code.contains("    var ts: Ts_Source { get }\n")
            && code.contains("    public typealias Ts_Source = CodecZenohTimestamp\n"),
        "the body is presented as a source of its codec, bound to its value: {code}"
    );
    assert!(
        !code.contains("ext_size: UInt64 = 0") && !code.contains("ext_size: ext_size"),
        "the value holds no length the encode derives: {code}"
    );
    assert!(
        code.contains("public static let frameLength = 1 + CodecZenohTimestamp.frameLength"),
        "the fewest bytes are the field's own and the embedded codec's: {code}"
    );
}

#[test]
fn an_embedded_codec_bounded_by_a_length_is_read_from_those_bytes_alone() {
    let code = swift_of("tests/forge/resources/codec_zenoh_timestamp_ext.scxml")
        .expect("a length-bounded embedded codec is written");
    assert!(
        code.contains(
            "let ts_inner_length = Int(clamping: ext_size)\n        var ts_inner = SceCursor(Array(try reader.peekSlice(ts_inner_length)))\n        let ts = try CodecZenohTimestamp.decode(&ts_inner)\n        try reader.advance(ts_inner_length)"
        ),
        "it is decoded from a cursor over the bytes the length gives, and the frame's own cursor moves past them: {code}"
    );
}

/// A bounded collection (RFC section synth-5-L) is a value over a fixed number of slots: an occupancy
/// array and a generation counter per slot, and a handle that packs a slot with its generation, the 16/16
/// layout every backend uses. Its element is the struct the element codec is generated as.
#[test]
fn a_bounded_collection_is_a_value_over_slots_with_a_packed_handle() {
    let code = swift_of("tests/forge/resources/local_sub_table.scxml")
        .expect("a bounded collection over a codec is written");
    assert!(
        code.contains("public struct LocalSubTable: Sequence, Sendable"),
        "{code}"
    );
    assert!(
        code.contains("private var slots: [SubscriptionEntry?]"),
        "the slots hold the element codec's struct: {code}"
    );
    assert!(
        code.contains("public static let capacity: Int = 8"),
        "the capacity is the document's: {code}"
    );
    assert!(
        code.contains("public struct LocalSubTableHandle: Hashable, Sendable"),
        "the handle is a value that packs a slot with its generation: {code}"
    );
    assert!(
        code.contains("return .full"),
        "a full collection answers `full` under a policy that does not evict: {code}"
    );
    assert!(
        !code.contains("public func findByIndex"),
        "a collection with no index declares no lookup by it: {code}"
    );
}

/// A content-line codec (SCE_FORGE.md section 4.6.4) is a struct of its entries read from the lines of
/// one component and written in declaration order. The line grammar is the runtime's: the generated
/// code asks the reader for each value and the writer to write one, and spells none of it.
#[test]
fn a_content_line_codec_is_a_struct_of_its_entries_read_and_written_by_the_runtime() {
    let code = swift_of("tests/forge/resources/codec_content_line_records.scxml")
        .expect("a content-line codec is written");
    assert!(
        code.contains("var reader = try ContentLineReader.begin(rest, component: \"VEVENT\")"),
        "a decode stands after the BEGIN of its component: {code}"
    );
    assert!(
        code.contains("var out = ContentLineWriter(component: \"VEVENT\")"),
        "an encode writes the lines of its component: {code}"
    );
    assert!(
        code.contains("public struct CodecContentLineRecordsAttendeeLine: Equatable, Sendable"),
        "a repeated property with a parameter is a list of line records: {code}"
    );
    assert!(
        code.contains("try cursor.advance(reader.consumed)"),
        "the cursor moves only when the component was read whole: {code}"
    );
}

/// An entry that is an enum is read and written by the text of its variants (docs/adr/0015): the
/// table is data of the codec, and an open enum holds any number its carrier does where a closed one
/// refuses a number it does not declare.
#[test]
fn a_content_line_enum_entry_is_read_by_the_text_of_its_variants() {
    let code = swift_of("tests/forge/resources/codec_content_line_enums.scxml")
        .expect("a content-line codec with enum entries is written");
    assert!(code.contains("private let TEXTS_"), "{code}");
    assert!(code.contains("try line.readEnum(TEXTS_"), "{code}");
    assert!(
        code.contains("throw CodecError.lineBadValue"),
        "a closed enum refuses a carrier it does not declare: {code}"
    );
    assert!(
        code.contains("try out.enumValue(TEXTS_"),
        "an enum value is written as the text of its variant: {code}"
    );
}

#[test]
fn a_cbor_codec_is_a_struct_of_its_entries_read_in_any_order_and_written_by_key() {
    let code = swift_of("tests/forge/resources/codec_cbor_map.scxml")
        .expect("a CBOR codec of scalars, text and bytes is written");
    // A required entry starts at its type's zero; an optional one is absent.
    assert!(
        code.contains("\n    public var source: String = \"\"\n"),
        "{code}"
    );
    assert!(
        code.contains("\n    public var deadline: UInt64? = nil\n"),
        "{code}"
    );
    // The decode takes each key where it finds it, skips one it does not declare, and holds a key
    // given twice to be no map this codec reads.
    assert!(
        code.contains("case 12:\n                // A key given twice is not one map entry.\n                if deadline_held != nil { throw CodecError.cborMalformed }\n                deadline_held = UInt64(truncatingIfNeeded: try Cbor.readUintUpto(&reader, 18446744073709551615))"),
        "{code}"
    );
    assert!(
        code.contains("default:\n                try Cbor.skip(&reader)"),
        "{code}"
    );
    assert!(
        code.contains(
            "guard let source_value = source_held else { throw CodecError.cborRequiredKeyMissing }"
        ),
        "a required entry the map lacks is refused: {code}"
    );
    // The encode writes the entries in ascending key order, whatever the order they were declared in.
    let encode = code
        .split("public func encode")
        .nth(1)
        .expect("the codec has an encode");
    let at = |key: &str| {
        encode
            .find(&format!("Cbor.writeUint({key}, to: &frame)"))
            .unwrap_or_else(|| panic!("the encode writes key {key}: {code}"))
    };
    assert!(
        at("0") < at("1")
            && at("1") < at("3")
            && at("3") < at("5")
            && at("5") < at("7")
            && at("7") < at("12"),
        "{code}"
    );
    assert!(
        code.contains(
            "var count: UInt64 = 4\n        if self.deadline != nil { count += 1 }\n        if self.urgent != nil { count += 1 }"
        ),
        "an absent entry is not counted: {code}"
    );
}

#[test]
fn a_cbor_entry_is_held_to_the_bound_its_document_declares_before_a_byte_is_written() {
    let code = swift_of("tests/forge/resources/codec_cbor_map.scxml")
        .expect("a CBOR codec of scalars, text and bytes is written");
    assert!(
        code.contains("if value.utf8.count > 64 { throw CodecError.cborOutOfRange }"),
        "a text is bounded in bytes: {code}"
    );
    assert!(
        code.contains("if value.count > 256 { throw CodecError.cborOutOfRange }"),
        "{code}"
    );
    assert!(
        code.contains("if value.count != 16 { throw CodecError.cborWrongLength }"),
        "a byte string of an exact length is not one of another: {code}"
    );
    assert!(
        code.contains("try sink.writeBytes(frame)"),
        "the map is handed to the sink in one write, so a refusal has written nothing: {code}"
    );
}

#[test]
fn a_cbor_enum_entry_is_read_through_its_carrier_and_refused_when_no_variant_declares_it() {
    let code = swift_of("stdlib/mesh/envelope.scxml")
        .expect("a CBOR codec with a closed enum entry is written");
    // A required enum starts at the first variant the document declares, and an optional one absent.
    assert!(
        code.contains("\n    public var pattern: PatternKind = PatternKind.fireForget\n"),
        "{code}"
    );
    assert!(
        code.contains("\n    public var rpc_status: RpcStatus? = nil\n"),
        "{code}"
    );
    // The carrier is read at its own width and converted by the enum's own function, which answers
    // nil for a number no variant declares: that number is no value of the entry.
    assert!(
        code.contains("guard let declared = PatternKind.fromUnderlying(UInt16(truncatingIfNeeded: try Cbor.readUintUpto(&reader, 65535))) else {\n                    throw CodecError.cborOutOfRange\n                }"),
        "{code}"
    );
    assert!(
        code.contains("Cbor.writeUint(UInt64(PatternKind.toUnderlying(value)), to: &frame)"),
        "{code}"
    );
}

/// A variant whose wire form carries no tag (ADR 0062) is told which arm a frame holds by its
/// caller: the tag is the last argument of a decode, a byte, and the arm is selected by it. An
/// encode is told nothing, since the arm the value holds says which it is.
#[test]
fn a_variant_whose_tag_its_caller_supplies_is_given_the_tag_on_decode() {
    let code = swift_of(
        "tests/forge/conformance/broadcast_log_context/broadcast_log_conformance_choice.scxml",
    )
    .expect("a variant with a caller's tag is written");
    assert!(
        code.contains(
            "public static func decode(_ cursor: inout SceCursor, tag: UInt8) throws(CodecError)"
        ),
        "the caller's tag is a parameter of the decode: {code}"
    );
    assert!(
        code.contains("let body_tag = tag"),
        "the arm is selected by the tag the caller gave: {code}"
    );
    assert!(
        code.contains(
            "public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError)"
        ),
        "an encode takes no tag: {code}"
    );
}

/// A codec that embeds a variant whose tag its caller supplies says which of its own flags selects
/// the arm (`<sce:variant-dispatch flag="hdr.kind"/>`), and hands the child that flag, shifted down
/// and masked, as the `tag` of its decode.
#[test]
fn an_embedded_variant_is_handed_its_tag_by_a_flag_of_the_embedding_codec() {
    let code = swift_of("tests/forge/resources/codec_source_parent_tag.scxml")
        .expect("a codec that embeds a variant by a dispatch flag is written");
    assert!(
        code.contains(
            "CodecSourceLeaf.decode(&reader, tag: UInt8(truncatingIfNeeded: (hdr >> 0) & 0x1))"
        ),
        "the tag is the bit of the carrier's flag: {code}"
    );
    // The encode derives the flag from the tag of the arm the body holds, read off the body's arms
    // enum, which names each arm's tag once.
    assert!(
        code.contains("        hdr_kind_filled |= UInt64(source.body.body.tag) & 0x1\n"),
        "the encode derives the flag from the arm that is held: {code}"
    );
    assert!(
        code.contains(
            "        hdr_folded = (hdr_folded & ~0x1) | ((UInt8(truncatingIfNeeded: hdr_kind_filled) & 0x1) << 0)\n"
        ),
        "the flag keeps the carrier's other bits: {code}"
    );
    let leaf = swift_of("tests/forge/resources/codec_source_leaf.scxml")
        .expect("a variant whose caller hands its tag is written");
    assert!(
        leaf.contains("        case .codecVariantSessionClose: return 0x1\n"),
        "an arm names the tag it is chosen by: {leaf}"
    );
}

/// An open enum (`sce:strict-variants="false"`) is a struct over its carrier that holds any number,
/// so a CBOR entry of one takes every number the carrier holds: the conversion is total, and there
/// is no number to refuse. The document is written beside the enum it imports.
#[test]
fn a_cbor_open_enum_entry_takes_every_number_its_carrier_holds() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::copy(
        repo_root().join("tests/forge/resources/enum_uds_nrc.scxml"),
        dir.path().join("enum_uds_nrc.scxml"),
    )
    .expect("copy the open enum");
    let document = dir.path().join("codec_cbor_open.scxml");
    std::fs::write(
        &document,
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:encoding="cbor" sce:codec-format="2" name="codec_cbor_open" version="1.0">
  <sce:import kind="enum" src="enum_uds_nrc.scxml" as="Nrc"/>
  <datamodel>
    <data id="raw" sce:type="bytes" sce:direction="in"/>
    <data id="status" sce:type="enum:Nrc" sce:key="1" sce:direction="out"/>
    <data id="detail" sce:type="optional&lt;enum:Nrc&gt;" sce:key="2" sce:direction="out"/>
  </datamodel>
</scxml>"#,
    )
    .expect("write the codec");
    let output = compile_forge_file(
        &document,
        Language::Swift,
        &[],
        &ForgeCompileOptions::default(),
    )
    .expect("a CBOR codec with an open enum entry is written");
    let (_, code) = output
        .files
        .into_iter()
        .next()
        .expect("a document that is written writes a file");
    assert!(
        code.contains(
            "status_held = EnumUdsNrc.fromUnderlying(UInt8(truncatingIfNeeded: try Cbor.readUintUpto(&reader, 255)))"
        ),
        "an open enum entry is every number the carrier holds: {code}"
    );
    assert!(
        !code.contains("guard let declared"),
        "an open enum has no number to refuse: {code}"
    );
    assert!(
        code.contains("Cbor.writeUint(UInt64(EnumUdsNrc.toUnderlying(value)), to: &frame)"),
        "{code}"
    );
}

#[test]
fn a_field_of_an_enum_is_carried_as_the_number_its_variants_declare() {
    let code = swift_of("tests/forge/resources/codec_enum_fields.scxml")
        .expect("a codec of enum fields at fixed places is written");
    for (expected, why) in [
        (
            "public var state: EnumSessionState = EnumSessionState.defaultSession",
            "a value starts at the first variant its enum declares, not at the carrier's zero",
        ),
        (
            "public var code: EnumUdsNrc = EnumUdsNrc.generalReject",
            "an open enum starts at its first declared variant as well",
        ),
        (
            "state: try declaredEnumValue(EnumSessionState.fromUnderlying(cursor.peekInteger(UInt8.self, at: 0, .big))),",
            "a closed enum refuses a number none of its variants declares",
        ),
        (
            "code: try EnumUdsNrc.fromUnderlying(cursor.peekInteger(UInt8.self, at: 1, .big)),",
            "an open enum holds every number its carrier does",
        ),
        (
            "storeInteger(EnumSessionState.toUnderlying(source.state), into: &frame, at: 0, .big)",
            "an encode writes the number the variant declares",
        ),
        (
            "storeInteger(EnumUdsNrc.toUnderlying(source.code), into: &frame, at: 1, .big)",
            "an open enum's value is written as its number",
        ),
        (
            "public static let frameLength = 3",
            "an enum field takes the bytes of its carrier",
        ),
    ] {
        assert!(code.contains(expected), "{why}: {expected} in {code}");
    }
}

#[test]
fn a_gated_field_of_an_enum_is_read_and_written_in_turn_where_its_flag_is_set() {
    let code = swift_of("tests/forge/resources/codec_enum_gated.scxml")
        .expect("a codec with a gated enum field is written");
    for (expected, why) in [
        (
            "public var state: EnumSessionState? = nil",
            "a gated enum field is optional",
        ),
        (
            "let code = try EnumUdsNrc.fromUnderlying(reader.readInteger(UInt8.self, .big))",
            "a frame read field by field reads the carrier from the cursor",
        ),
        (
            "if (flags & 0x1) != 0 {\n            state = try declaredEnumValue(EnumSessionState.fromUnderlying(reader.readInteger(UInt8.self, .big)))\n        }",
            "the gated field is read where its flag is set, and refused for an undeclared number",
        ),
        (
            "appendInteger(EnumUdsNrc.toUnderlying(self.code), to: &frame, .big)",
            "an encode appends the number",
        ),
        (
            "if let state = self.state {\n            appendInteger(EnumSessionState.toUnderlying(state), to: &frame, .big)\n        }",
            "a gated enum field is appended where it is held",
        ),
    ] {
        assert!(code.contains(expected), "{why}: {expected} in {code}");
    }
}

/// docs/adr/0065 §6 R4e: an input one arm of two says is said by that arm and by nothing in the
/// other, so its reader answers `nil` there; the parent that hands it a bit of its own carrier holds
/// the arm to that bit where the arm says it, and refuses nothing where it says nothing.
#[test]
fn an_input_some_arms_say_is_said_by_the_arm_that_says_it_and_held_where_it_is() {
    let code = swift_of("tests/forge/resources/codec_some_arms_say.scxml")
        .expect("a variant one arm of which says its input is written");
    assert!(
        code.contains(
            "public static func inputN<Source: CodecSomeArmsSaySource>(_ source: Source) -> UInt8? {\n        switch source.body {\n        case .codecSomeArmsSayPlain: return nil\n        case .codecEveryArmSaysNear(let arm): return CodecEveryArmSaysNear.inputN(arm)\n        }\n    }"
        ),
        "{code}"
    );
    let parent = swift_of("tests/forge/resources/codec_some_arms_say_parent.scxml")
        .expect("a codec that hands a bit to a body that says it in some values is written");
    assert!(
        parent.contains(
            "    if (CodecSomeArmsSay.inputN(source.body).map { ($0 != 0) != handed } ?? false) { throw CodecError.presentIfMismatch }"
        ),
        "{parent}"
    );
}

#[test]
fn an_arm_under_a_feature_is_a_case_of_the_build_that_has_it_and_its_tag_is_refused_elsewhere() {
    let code = swift_of("backends/rust/forge-conformance/fixtures/codec_feature_gated_arm.scxml")
        .expect("a variant with an arm under a feature is written");
    for (expected, why) in [
        (
            "#if !SCE_FEATURE_GATED_ARM_OFF\n    case codecDefaultMarkerArmB(",
            "the arm's case is declared unless the build sets the condition",
        ),
        (
            "        #if !SCE_FEATURE_GATED_ARM_OFF\n        case 0x2:\n            body = .codecDefaultMarkerArmB(try ",
            "the decode reads the arm in the build that has it",
        ),
        (
            "        #else\n        case 0x2:\n            throw CodecError.variantArmNotBuilt\n        #endif",
            "and refuses its tag in the build that leaves it out, not reading it as the catch-all",
        ),
        (
            "        #if !SCE_FEATURE_GATED_ARM_OFF\n        case .codecDefaultMarkerArmB(let arm):\n            try CodecDefaultMarkerArmB.encodeSource(arm, into: &body_arm)\n        #endif",
            "the printed encode writes the arm in the build that has it",
        ),
        (
            "        #if !SCE_FEATURE_GATED_ARM_OFF\n        case .codecDefaultMarkerArmB: return 0x2\n        #endif",
            "the arms enum names the arm's tag in the build that has it",
        ),
        (
            "public var body: CodecFeatureGatedArmVariant = .codecDefaultMarkerArmA(",
            "a value starts at the arm the document marks as its default, which every build holds",
        ),
    ] {
        assert!(code.contains(expected), "{why}: {expected} in {code}");
    }
    assert!(
        !code.contains("#if !SCE_FEATURE_GATED_ARM_OFF\n    case codecDefaultMarkerArmA("),
        "an arm under no feature is in every build: {code}"
    );

    let last =
        swift_of("backends/rust/forge-conformance/fixtures/codec_feature_gated_arm_held.scxml")
            .expect(
                "a variant whose last arm is under a feature and that has no catch-all is written",
            );
    assert!(
        last.contains(
            "        #else\n        default:\n            throw CodecError.variantArmNotBuilt\n        #endif"
        ),
        "the last arm stands for what the others do not match, and is refused where it is \
         not built: {last}"
    );
}

#[test]
fn a_field_of_fewer_whole_bytes_than_its_type_is_read_and_written_as_those_bytes_in_turn() {
    let code = swift_of_datamodel(
        r#"<sce:field id="n" sce:type="uint32" sce:byte="0" sce:bit-size="24"/>
    <sce:field id="body" sce:type="bytes" sce:byte="3" sce:bit-size="tail" sce:max-size="8"/>"#,
    )
    .expect("whole bytes fewer than the type's are written in a frame read field by field");
    for (expected, why) in [
        (
            "let n = try reader.readInteger(UInt32.self, bytes: 3, .big)",
            "the decode reads the three bytes the field has and no fourth",
        ),
        (
            "frame[0] |= UInt8(truncatingIfNeeded: (source.n >> 16) & 0xFF) << 0",
            "the printed encode writes the three bytes of the frame's fixed part and cuts what \
             is above them",
        ),
        (
            "public static let frameLength = 3",
            "the fewest bytes count the field's own three",
        ),
    ] {
        assert!(code.contains(expected), "{why}: {expected} in {code}");
    }
}
