// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A codec (`sce:kind="codec"`, a frame of fields at byte offsets), written for Swift in part: a
// positional codec of native-width integers is a struct with a `decode` over the runtime's
// `SceCursor` and an `encode` into its `SceSink`, and what the Swift codec does not write yet (a
// bit field, a flag carrier, a repeat, a variant, a string, another encoding) is refused by name
// and never written without it.
//
// The runtime half is `backends/swift/forge-runtime/Sources/SceForgeRuntime/Codec.swift`, held by
// its own tests; this holds what the generator writes against it.

use std::path::{Path, PathBuf};

use sce_build::generator::Language;
use sce_build::{compile_forge_file, ForgeCompileOptions};

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
fn what_the_swift_codec_does_not_write_is_refused_by_name() {
    for (document, what) in [
        ("codec_cbor_map", "cbor"),
        ("codec_content_line_enums", "content-line"),
        ("codec_subbyte", "field '"),
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
