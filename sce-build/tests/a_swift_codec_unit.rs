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
        code.contains("storeInteger(header, into: &frame, at: 0, .big)"),
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
        code.contains("storeInteger(payload, into: &frame, at: 2, .big)"),
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
        code.contains("storeInteger(value, into: &frame, at: 1, .little)"),
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
        code.contains("frame[0] |= (channel & 0x7) << 2"),
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
fn what_the_swift_codec_does_not_write_is_refused_by_name() {
    for (document, what) in [
        ("codec_cbor_map", "cbor"),
        ("codec_content_line_enums", "content-line"),
        ("codec_chain_flag_carrier", "field '"),
        ("codec_chain_flag_entry", "a flag input"),
        ("codec_chain_has_envelope", "field '"),
        ("codec_embed_basic", "field '"),
    ] {
        let refusal = swift_of(&format!("tests/forge/resources/{document}.scxml"))
            .expect_err(&format!("{document} must be refused, not written"));
        assert!(
            refusal.contains("has no Swift lowering yet") && refusal.contains(what),
            "{document} is refused, but not by name ({what}): {refusal}"
        );
    }
}
