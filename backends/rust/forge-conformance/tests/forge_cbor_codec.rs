// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// SCE_FORGE.md §4.6.1 — a `sce:encoding="cbor"` codec, generated for Rust
// from `tests/forge/resources/codec_cbor_map.scxml` and held to the bytes
// RFC 8949 fixes for its values: the shortest heads, the entries present in
// ascending key order, keys read in any order, an unknown key skipped whole,
// and every refusal the section names — each leaving the cursor where it was.
#![cfg(feature = "alloc")]

mod codec_cbor_map {
    include!(concat!(env!("OUT_DIR"), "/codec_cbor_map.rs"));
}

// The Mesh envelope (`tests/forge/resources/mesh_envelope.scxml`) and the
// standard enums it imports, as siblings — the envelope names them
// `super::pattern_kind` and so on.
mod mesh {
    pub mod pattern_kind {
        include!(concat!(env!("OUT_DIR"), "/pattern_kind.rs"));
    }
    pub mod payload_codec {
        include!(concat!(env!("OUT_DIR"), "/payload_codec.rs"));
    }
    pub mod rpc_status {
        include!(concat!(env!("OUT_DIR"), "/rpc_status.rs"));
    }
    pub mod mesh_envelope {
        include!(concat!(env!("OUT_DIR"), "/mesh_envelope.rs"));
    }
}

use codec_cbor_map::CodecCborMap;
use sce_forge_runtime::codec::{CodecError, SceCursor};

const ID: [u8; 16] = [
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
];

fn value() -> CodecCborMap<'static> {
    CodecCborMap {
        source: "node-a",
        id: &ID,
        pattern: 3,
        payload: &[0xde, 0xad],
        deadline: None,
        urgent: Some(true),
    }
}

/// `{0: h'000102…0f', 1: "node-a", 3: 3, 5: h'dead', 7: true}` as RFC 8949
/// writes it deterministically.
fn expected() -> Vec<u8> {
    let mut bytes = vec![0xa5, 0x00, 0x50];
    bytes.extend_from_slice(&ID);
    bytes.extend_from_slice(&[0x01, 0x66]);
    bytes.extend_from_slice(b"node-a");
    bytes.extend_from_slice(&[0x03, 0x03, 0x05, 0x42, 0xde, 0xad, 0x07, 0xf5]);
    bytes
}

fn decode(bytes: &[u8]) -> Result<CodecCborMap<'_>, CodecError> {
    CodecCborMap::decode(&mut SceCursor::new(bytes))
}

#[test]
fn a_value_is_written_in_its_one_deterministic_encoding() {
    assert_eq!(value().encode_to_vec().expect("encodes"), expected());
}

#[test]
fn an_optional_entry_is_written_only_when_present_and_in_key_order() {
    let mut v = value();
    v.deadline = Some(1 << 32);
    let mut want = expected();
    want[0] = 0xa6;
    want.extend_from_slice(&[0x0c, 0x1b, 0, 0, 0, 1, 0, 0, 0, 0]);
    assert_eq!(v.encode_to_vec().expect("encodes"), want);
}

#[test]
fn what_was_written_is_read_back() {
    assert_eq!(decode(&expected()), Ok(value()));
}

#[test]
fn keys_are_read_in_any_order_and_an_unknown_one_is_skipped_whole() {
    // {7: true, 20: [1, {2: h'00'}], 5: h'dead', 3: 3, 1: "node-a", 0: id}
    let mut bytes = vec![0xa6, 0x07, 0xf5, 0x14, 0x82, 0x01, 0xa1, 0x02, 0x41, 0x00];
    bytes.extend_from_slice(&[0x05, 0x42, 0xde, 0xad, 0x03, 0x03, 0x01, 0x66]);
    bytes.extend_from_slice(b"node-a");
    bytes.extend_from_slice(&[0x00, 0x50]);
    bytes.extend_from_slice(&ID);
    assert_eq!(decode(&bytes), Ok(value()));
}

#[test]
fn a_head_longer_than_it_needs_is_still_read() {
    // `pattern` written as 0x18 0x03 rather than 0x03.
    let mut bytes = expected();
    let at = bytes
        .windows(2)
        .position(|w| w == [0x03, 0x03])
        .expect("3: 3");
    bytes.splice(at + 1..at + 2, [0x18, 0x03]);
    assert_eq!(decode(&bytes), Ok(value()));
}

#[test]
fn every_refusal_the_section_names_leaves_the_cursor_where_it_was() {
    let refused = |bytes: &[u8], error: CodecError| {
        let mut cursor = SceCursor::new(bytes);
        assert_eq!(
            CodecCborMap::decode(&mut cursor),
            Err(error),
            "{bytes:02x?}"
        );
        assert_eq!(
            cursor.remaining(),
            bytes.len(),
            "the cursor moved on {error:?}"
        );
    };

    // A required key absent: drop `3: 3` and say four entries.
    let mut missing = expected();
    missing[0] = 0xa4;
    let at = missing
        .windows(2)
        .position(|w| w == [0x03, 0x03])
        .expect("3: 3");
    missing.drain(at..at + 2);
    refused(&missing, CodecError::CborRequiredKeyMissing);

    // The id one byte short of its exact length.
    let mut short = vec![0xa5, 0x00, 0x4f];
    short.extend_from_slice(&ID[..15]);
    short.extend_from_slice(&expected()[19..]);
    refused(&short, CodecError::CborWrongLength);

    // A key given twice.
    let mut twice = expected();
    twice[0] = 0xa6;
    twice.extend_from_slice(&[0x07, 0xf4]);
    refused(&twice, CodecError::CborMalformed);

    // `pattern` holding 300, past a uint8.
    let mut wide = expected();
    let at = wide
        .windows(2)
        .position(|w| w == [0x03, 0x03])
        .expect("3: 3");
    wide.splice(at + 1..at + 2, [0x19, 0x01, 0x2c]);
    refused(&wide, CodecError::CborOutOfRange);

    // A text where the map expected a truth value.
    let mut kind = expected();
    let last = kind.len() - 1;
    kind.splice(last..last + 1, [0x61, b'x']);
    refused(&kind, CodecError::CborMalformed);

    // An indefinite-length map.
    refused(&[0xbf, 0xff], CodecError::CborMalformed);
}

/// The C++ codec's `GoldenBytesForFixedEnvelope`
/// (tests/mesh/MeshEnvelopeCodecTest.cpp): the same envelope must be the same
/// bytes whichever backend wrote it — the generated codec replaces the
/// hand-written one only if this holds.
#[test]
fn the_mesh_envelope_writes_the_bytes_the_cpp_codec_writes() {
    use mesh::mesh_envelope::MeshEnvelope as Envelope;
    use mesh::pattern_kind::PatternKind;
    use mesh::payload_codec::PayloadCodec;

    const SAMPLE_ID: [u8; 16] = [
        0x01, 0x82, 0xb1, 0x4d, 0xa3, 0x5c, 0x70, 0x12, 0xb4, 0xde, 0xf0, 0x42, 0x9a, 0x88, 0x77,
        0x66,
    ];
    let envelope = Envelope {
        id: &SAMPLE_ID,
        source: "ecu",
        event_type: "evt",
        pattern: PatternKind::FireForget,
        datacontenttype: PayloadCodec::Json,
        data: &[0xAA, 0xBB],
        ..Envelope::default()
    };
    let mut golden = vec![0xA6, 0x00, 0x50];
    golden.extend_from_slice(&SAMPLE_ID);
    golden.extend_from_slice(&[0x01, 0x63, b'e', b'c', b'u']);
    golden.extend_from_slice(&[0x02, 0x63, b'e', b'v', b't']);
    golden.extend_from_slice(&[0x03, 0x01, 0x04, 0x01, 0x05, 0x42, 0xAA, 0xBB]);

    assert_eq!(envelope.encode_to_vec().expect("encodes"), golden);
    assert_eq!(
        Envelope::decode(&mut SceCursor::new(&golden)),
        Ok(envelope.clone())
    );

    // A pattern in the reserved Stream range (10–13) is not a value of the
    // closed set, as the C++ decoder refuses it.
    let mut reserved = golden.clone();
    let at = reserved
        .windows(2)
        .position(|w| w == [0x03, 0x01])
        .expect("3: 1");
    reserved[at + 1] = 0x0b;
    assert_eq!(
        Envelope::decode(&mut SceCursor::new(&reserved)),
        Err(CodecError::UndeclaredEnumValue)
    );
}

#[test]
fn a_value_that_breaks_its_bound_is_not_written() {
    let mut v = value();
    v.id = &ID[..15];
    assert_eq!(v.encode_to_vec(), Err(CodecError::CborWrongLength));

    let long = "x".repeat(65);
    let mut v = value();
    v.source = &long;
    assert_eq!(v.encode_to_vec(), Err(CodecError::CborOutOfRange));
}
