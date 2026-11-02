// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
        code.contains("storeInteger(self.header, into: &frame, at: 0, .big)"),
        "the flags do not change the wire: {code}"
    );
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
        code.contains("public struct CodecSimpleFrame: Equatable, Sendable {"),
        "{code}"
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
        code.contains("storeInteger(self.payload, into: &frame, at: 2, .big)"),
        "and written at the same place: {code}"
    );
}

#[test]
fn a_field_is_read_and_written_in_the_documents_byte_order() {
    let code = swift_of("tests/forge/resources/codec_little_endian.scxml")
        .expect("a codec of integers is written");
    assert!(
        code.contains("public struct CodecLittleEndian: Equatable, Sendable {"),
        "{code}"
    );
    assert!(
        code.contains("value: try cursor.peekInteger(UInt16.self, at: 1, .little)"),
        "{code}"
    );
    assert!(
        code.contains("storeInteger(self.value, into: &frame, at: 1, .little)"),
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
fn an_encode_reads_a_field_through_self_so_a_local_cannot_hide_it() {
    // The encode lays the frame out in a local named `frame` and writes to a parameter named
    // `sink`; a field of either name, read bare, would be read as the local or the parameter.
    let code = swift_of_datamodel(
        r#"<sce:field id="frame" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
           <sce:field id="sink" sce:type="uint8" sce:byte="1" sce:bit-size="8"/>
           <sce:field id="low" sce:type="uint8" sce:byte="2" sce:bit-offset="0" sce:bit-size="4"/>"#,
    )
    .expect("fields named for the encode's own names are written");
    assert!(
        code.contains("storeInteger(self.frame, into: &frame, at: 0, .big)"),
        "{code}"
    );
    assert!(
        code.contains("storeInteger(self.sink, into: &frame, at: 1, .big)"),
        "{code}"
    );
    assert!(code.contains("frame[2] |= (self.low & 0xF) << 0"), "{code}");
}

#[test]
fn a_tail_is_the_bytes_to_the_end_of_the_frame() {
    let code = swift_of("tests/forge/resources/codec_tail.scxml").expect("a tail is written");
    assert!(code.contains("public var payload: [UInt8] = []"), "{code}");
    assert!(
        code.contains("public static let frameLength = 2"),
        "the fixed part is the bytes before the tail: {code}"
    );
    assert!(
        code.contains("var reader = cursor"),
        "the frame is read through a copy of the cursor: {code}"
    );
    assert!(
        code.contains("frame.payload = Array(try reader.readSlice(reader.remaining))"),
        "{code}"
    );
    assert!(code.contains("cursor = reader"), "{code}");
    assert!(
        code.contains("frame += self.payload"),
        "the encode appends the bytes after the fixed part: {code}"
    );
}

#[test]
fn a_variable_length_integer_is_read_and_written_as_a_chain() {
    let code =
        swift_of("tests/forge/resources/codec_vle_zint_u64.scxml").expect("a VLE is written");
    assert!(code.contains("public var value: UInt64 = 0"), "{code}");
    assert!(
        code.contains("frame.value = try reader.readVle(UInt64.self)"),
        "{code}"
    );
    assert!(code.contains("appendVle(self.value, to: &frame)"), "{code}");
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
        .find("frame.time = try reader.readVle")
        .expect("the chain is read");
    let size = code
        .find("frame.zid_size = try reader.readInteger")
        .expect("the size is read");
    let zid = code
        .find("frame.zid = Array(try reader.readSlice")
        .expect("the bytes are read");
    assert!(
        time < size && size < zid,
        "the fields are read in the order of the document: {code}"
    );
    let fixed_after = swift_of("tests/forge/resources/codec_fixed_after_lengthref.scxml")
        .expect("a fixed field after a length-ref is written");
    let bytes = fixed_after
        .find("frame.payload = Array")
        .expect("the bytes are read");
    let crc = fixed_after
        .find("frame.crc32 = try reader.readInteger")
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
        code.contains(
            "frame.payload = Array(try reader.readSlice(Int(clamping: frame.payload_len)))"
        ),
        "{code}"
    );
    let arith = swift_of("tests/forge/resources/codec_scout_zid_body.scxml")
        .expect("a length-ref with arithmetic is written");
    assert!(
        arith.contains("Int(clamping: frame.zid_len_m1) &+ (1)"),
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
        tail.contains("if reader.remaining > 8 {\n            throw CodecError.lengthOutOfRange\n        }\n        frame.body = Array(try reader.readSlice(reader.remaining))"),
        "a decode refuses the bytes that are left before it cuts them: {tail}"
    );
    assert!(
        tail.contains("if self.body.count > 8 {\n            throw CodecError.lengthOutOfRange\n        }\n        var frame: [UInt8] = []"),
        "an encode refuses a value past the bound before its first byte: {tail}"
    );
    let counted = swift_of_datamodel(
        r#"<sce:field id="tag_len" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
           <sce:field id="tag" sce:type="bytes" sce:byte="1" sce:bit-size="length-ref" sce:length-field="tag_len" sce:length-arith="-1" sce:max-size="4"/>"#,
    )
    .expect("a bounded length-ref is written");
    assert!(
        counted.contains("if frame.tag_len < 1 || frame.tag_len > 5 {"),
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
        code.contains("\n    public var name: String = \"\"\n"),
        "a field is declared on a line of its own, after its comment: {code}"
    );
    assert!(
        code.contains("frame.name = try reader.readString(Int(clamping: frame.n))"),
        "a decode reads the bytes as UTF-8 text, refusing bytes that are not: {code}"
    );
    assert!(
        code.contains("frame += Array(self.name.utf8)"),
        "an encode writes the UTF-8 bytes of the text: {code}"
    );
    assert!(
        code.contains("if self.name.utf8.count > 8 {"),
        "the bound counts the bytes on the wire, not the characters: {code}"
    );
}

#[test]
fn a_gated_text_reads_a_length_that_shares_its_gate_through_the_value_it_has_there() {
    let code = swift_of("tests/forge/resources/codec_present_if_string.scxml")
        .expect("a gated text with a gated length is written");
    assert!(code.contains("public var text: String? = nil"), "{code}");
    assert!(
        code.contains("try reader.readString(Int(clamping: (frame.text_len ?? 0)))"),
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
        code.contains("public enum CodecVariantDispatchVariant: Equatable, Sendable {\n    case codecVariantSessionOpen(CodecVariantSessionOpen)\n    case codecVariantSessionClose(CodecVariantSessionClose)\n    case `default`(tag: UInt8, body: CodecVariantSessionClose)\n}"),
        "{code}"
    );
    assert!(
        code.contains("\n    public var body: CodecVariantDispatchVariant = .codecVariantSessionClose(CodecVariantSessionClose())\n"),
        "the arm the document marks as its default is what a value that has read no frame holds: {code}"
    );
    assert!(
        code.contains("let body_tag = frame.msg_id\n        switch body_tag {\n        case 0x1:\n            frame.body = .codecVariantSessionOpen(try CodecVariantSessionOpen.decode(&reader))\n        case 0x2:"),
        "{code}"
    );
    assert!(
        code.contains("        default:\n            frame.body = .default(tag: body_tag, body: try CodecVariantSessionClose.decode(&reader))"),
        "{code}"
    );
    assert!(
        code.contains("case .default(_, let arm):\n                try arm.encode(into: &piece)"),
        "an encode writes the arm that is held: {code}"
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
        code.contains("let body_tag = (frame.header >> 0) & 0x1F"),
        "{code}"
    );
    assert!(
        code.contains("frame.body = .default(tag: UInt8(truncatingIfNeeded: body_tag), body: try CodecZenohPut.decode(&reader))"),
        "the tag the catch-all holds is the bits, in the smallest type that holds them: {code}"
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
            "default:\n            frame.body = .codecPeekArmB(try CodecPeekArmB.decode(&reader))"
        ),
        "no catch-all: the last arm stands for what is left: {code}"
    );
}

#[test]
fn an_embedded_variant_whose_arm_the_embedder_drives_is_refused_by_name() {
    // The importer declares a dispatch that writes the arm into one of its own flags at encode, which
    // the Swift codec does not do, so the codec is not written rather than written without it.
    let refusal = swift_of("tests/forge/resources/codec_embed_variant_dispatch.scxml")
        .expect_err("an embed whose arm is driven into a flag is not written");
    assert!(
        refusal.contains("its arm is driven into a flag of the codec that embeds it"),
        "{refusal}"
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
        code.contains(
            "public func encode<Sink: SceSink>(into sink: inout Sink, N: UInt8) throws(CodecError)"
        ),
        "{code}"
    );
    assert!(
        code.contains("        if (N & 0x1) != 0 {\n            frame.suffix_len = "),
        "a decode reads a field gated by the input only when the input is set: {code}"
    );
}

#[test]
fn an_encode_refuses_a_field_and_an_input_that_contradict_each_other_before_a_byte() {
    let code = swift_of("tests/forge/resources/codec_zenoh_wireexpr.scxml")
        .expect("a codec with a flag input is written");
    assert!(
        code.contains("if (N & 0x1) != 0 {\n            if self.suffix == nil {\n                throw CodecError.presentIfMismatch\n            }\n        } else if self.suffix != nil {\n            throw CodecError.presentIfMismatch\n        }\n"),
        "{code}"
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
    assert!(
        forwarded.contains("try self.wireexpr.encode(into: &piece, N: N)"),
        "{forwarded}"
    );
    let carrier = swift_of("tests/forge/resources/codec_repeat_flag_carrier.scxml")
        .expect("a list whose entries are handed a bit of a carrier is written");
    assert!(
        carrier.contains("wide: UInt8(truncatingIfNeeded: (frame.head >> 0) & 0x1)"),
        "a decode hands the bit of the carrier it has read: {carrier}"
    );
    assert!(
        carrier.contains("wide: UInt8(truncatingIfNeeded: (self.head >> 0) & 0x1)"),
        "an encode hands the bit of the carrier the value holds: {carrier}"
    );
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
            r#"<sce:field id="a" sce:type="uint8" sce:byte="0" sce:bit-offset="0" sce:bit-size="4"/>
               <sce:field id="n" sce:type="uint64" sce:byte="1" sce:bit-size="vle"/>"#,
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
        !code.contains("if ((frame.carrier >> 4) & 0xF)"),
        "four bits cannot say more than the 15 bytes the document admits, so nothing is judged: {code}"
    );
    assert!(
        code.contains("if self.payload.count > 15 {"),
        "an encode still refuses a value past the bound: {code}"
    );
    assert!(
        code.contains(
            "frame.payload = Array(try reader.readSlice(Int(clamping: ((frame.carrier >> 4) & 0xF))))"
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
        code.contains("frame[0] |= (self.channel & 0x7) << 2"),
        "an encode cuts the value to its width and ors it into the byte: {code}"
    );
    assert!(
        code.contains("public static let frameLength = 1"),
        "three fields of one byte are one byte: {code}"
    );
}

#[test]
fn a_bit_field_that_does_not_fit_in_one_byte_is_refused_by_name() {
    for (field, what) in [
        (
            r#"<sce:field id="wide" sce:type="uint16" sce:byte="0" sce:bit-offset="0" sce:bit-size="4"/>"#,
            "which only a `uint8` takes",
        ),
        (
            r#"<sce:field id="past" sce:type="uint8" sce:byte="0" sce:bit-offset="6" sce:bit-size="4"/>"#,
            "which are not inside one byte",
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
fn a_field_that_is_on_the_wire_on_a_flag_is_optional_and_read_only_when_the_flag_is_set() {
    let code = swift_of("tests/forge/resources/codec_present_if_basic.scxml")
        .expect("a gated field is written");
    assert!(code.contains("public var seq: UInt16? = nil"), "{code}");
    assert!(
        code.contains(
            "if (frame.flags & 0x1) != 0 {\n            frame.seq = try reader.readInteger(UInt16.self, .big)\n        }"
        ),
        "a decode reads the field only when the bit of the carrier it has read is set: {code}"
    );
    assert!(
        code.contains(
            "if let seq = self.seq {\n            appendInteger(seq, to: &frame, .big)\n        }"
        ),
        "an encode writes the field when it is given: {code}"
    );
    assert!(
        code.contains("public static let frameLength = 1"),
        "a field that may be absent adds nothing to the fewest bytes: {code}"
    );
}

#[test]
fn a_negated_clause_is_on_a_clear_bit_and_a_disjunction_on_any_clause() {
    let negated = swift_of("tests/forge/resources/codec_present_if_negation.scxml")
        .expect("a negated gate is written");
    assert!(negated.contains("(frame.flags & 0x1) == 0"), "{negated}");
    let either = swift_of("tests/forge/resources/codec_present_if_disjunction.scxml")
        .expect("a disjunction is written");
    assert!(
        either.contains("(frame.flags & 0x1) != 0 || (frame.flags & 0x2) != 0"),
        "any clause of a disjunction puts the field on the wire: {either}"
    );
}

#[test]
fn a_gated_variable_length_field_is_an_optional_that_reads_and_writes_as_an_ungated_one() {
    let tail = swift_of("tests/forge/resources/codec_present_if_tail.scxml")
        .expect("a gated tail is written");
    assert!(tail.contains(": [UInt8]? = nil"), "{tail}");
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
        code.contains("let _has_entries_7 = frame.entries.contains(where: { _e in chainIdentifier(_e.entry_type) == 7 })"),
        "the chain is asked for the identifier it holds, once it is read: {code}"
    );
    assert!(
        code.contains("if _has_entries_7 || (frame.header & 0x1) != 0 {"),
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
            "CodecChainPrevEntry.decode(&reader, after_marker: _prev_entries_after_marker, tagged: UInt8(truncatingIfNeeded: (frame.head >> 0) & 0x1))"
        ),
        "the chain's input and the carrier's bit are passed in the entry's declared order: {code}"
    );
    assert!(
        code.contains("_prev_entries_after_marker = (chainIdentifier(entry.extId) == 4) ? 1 : 0"),
        "it is rewritten from the entry just handled: {code}"
    );
    let encode = code
        .split("public func encode")
        .nth(1)
        .expect("the codec has an encode");
    assert!(
        encode.contains("try entry.encode(into: &piece, after_marker: _prev_entries_after_marker,")
            && encode.contains(
                "_prev_entries_after_marker = (chainIdentifier(entry.extId) == 4) ? 1 : 0"
            ),
        "an encode walks the same values over the same entries: {code}"
    );
}

#[test]
fn a_chain_to_the_end_of_the_frame_is_read_while_bytes_are_left_to_its_depth() {
    let code = swift_of("tests/forge/resources/codec_tlv_chain_basic.scxml")
        .expect("a chain to the end of the frame is written");
    assert!(
        code.contains("\n    public var extensions: [CodecTlvEntry] = []\n"),
        "{code}"
    );
    assert!(
        code.contains("for _ in 0..<8 {\n            if reader.remaining == 0 { break }\n            let entry = try CodecTlvEntry.decode(&reader)\n            extensions_items.append(entry)\n        }\n        if reader.remaining > 0 {\n            throw CodecError.tooManyElements\n        }"),
        "an entry past the depth is refused and not left on the wire unread: {code}"
    );
    assert!(
        code.contains(
            "if self.extensions.count > 8 {\n            throw CodecError.tooManyElements"
        ),
        "an encode refuses a chain past its depth: {code}"
    );
}

#[test]
fn a_chain_closed_by_an_entry_flag_is_read_until_an_entry_clears_it() {
    let code = swift_of("tests/forge/resources/codec_zenoh_err.scxml")
        .expect("a chain closed by an entry flag is written");
    assert!(
        code.contains("public var extensions: [CodecZenohExtEntry]? = nil"),
        "a gated chain is optional: {code}"
    );
    assert!(
        code.contains("let entry = try CodecZenohExtEntry.decode(&reader)\n                extensions_more = entry.Z\n                extensions_items.append(entry)\n                if !extensions_more { break }"),
        "{code}"
    );
    assert!(
        code.contains("if extensions_more && reader.remaining == 0 {\n                throw CodecError.needMoreBytes\n            }\n            if extensions_more {\n                throw CodecError.tooManyElements\n            }"),
        "an entry that promises another leaves the frame short, or is one past the depth: {code}"
    );
    assert!(
        code.contains("value.count < 1 || value.count > 4"),
        "a chain closed by a flag holds at least one entry: {code}"
    );
}

#[test]
fn a_list_is_an_array_of_the_struct_its_entry_codec_is_written_as() {
    let code = swift_of("tests/forge/resources/codec_repeat_unconditional_count.scxml")
        .expect("a list is written");
    assert!(
        code.contains("public var links: [CodecRepeatElem] = []"),
        "{code}"
    );
    assert!(
        code.contains("public var weights: [CodecRepeatElem]? = nil"),
        "a gated list is optional: {code}"
    );
    assert!(
        code.contains(
            "for _ in 0..<Int(clamping: frame.links_len) {\n            links_items.append(try CodecRepeatElem.decode(&reader))\n        }\n        frame.links = links_items"
        ),
        "a decode reads as many entries as the count says, into a local set when whole: {code}"
    );
    assert!(
        code.contains(
            "for entry in self.links {\n            var piece = BytesSink()\n            try entry.encode(into: &piece)\n            frame += piece.bytes\n        }"
        ),
        "an encode writes each entry in turn: {code}"
    );
    assert!(
        code.contains("public static let frameLength = 2"),
        "a list adds nothing to the fewest bytes: {code}"
    );
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
        code.contains("if (frame.num_elems ?? 0) > 32 {\n                throw CodecError.tooManyElements\n            }\n            var elems_items"),
        "the count is judged before an entry is read, from the count a gate may leave absent: {code}"
    );
    assert!(
        code.contains("if let value = self.elems, value.count > 32 {\n            throw CodecError.tooManyElements\n        }"),
        "an encode refuses a list past the bound before its first byte: {code}"
    );
}

#[test]
fn an_embedded_codec_swift_does_not_write_is_refused_by_name() {
    // The embedded codec dispatches on a tag, which the Swift codec does not write yet, so the
    // codec that embeds it names a type that is not there.
    let refusal = swift_of("tests/forge/resources/codec_embed_variant_dispatch.scxml")
        .expect_err("an embedded codec Swift does not write is not written");
    assert!(refusal.contains("which has no Swift lowering"), "{refusal}");
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
        code.contains("frame.locator = try CodecZenohLocator.decode(&reader)"),
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
    assert!(
        code.contains(
            "do {\n            var piece = BytesSink()\n            try self.ts.encode(into: &piece)\n            frame += piece.bytes\n        }"
        ),
        "an encode writes its bytes where the field stands: {code}"
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
            "let ts_inner_length = Int(clamping: frame.ext_size)\n        var ts_inner = SceCursor(Array(try reader.peekSlice(ts_inner_length)))\n        frame.ts = try CodecZenohTimestamp.decode(&ts_inner)\n        try reader.advance(ts_inner_length)"
        ),
        "it is decoded from a cursor over the bytes the length gives, and the frame's own cursor moves past them: {code}"
    );
}

#[test]
fn what_the_swift_codec_does_not_write_is_refused_by_name() {
    let refusal = swift_of("tests/forge/resources/codec_content_line_enums.scxml")
        .expect_err("a content-line codec must be refused, not written");
    assert!(
        refusal.contains("has no Swift lowering yet") && refusal.contains("content-line"),
        "a content-line codec is refused, but not by name: {refusal}"
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
fn a_cbor_codec_that_has_an_enum_entry_is_refused_by_name() {
    let refusal = swift_of("stdlib/mesh/envelope.scxml")
        .expect_err("a CBOR codec with an enum entry is not written");
    assert!(
        refusal.contains("an enum") && refusal.contains("has no Swift lowering yet"),
        "{refusal}"
    );
}
