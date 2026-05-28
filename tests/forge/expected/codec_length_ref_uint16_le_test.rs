// SCE Forge: Auto-generated codec test-vector sidecar (RFC §5.B B5-θ)
// Companion to codec_length_ref_uint16_le.rs — do not edit; regenerate from the source SCXML.

#[test]
fn test_vector_codec_length_ref_uint16_le_l25() {
    let actual = CodecLengthRefUint16Le {
        payload_len: 0x0u16,
        payload: b"",
    };
    let encoded = actual.encode_to_vec();
    let expected: &[u8] = &[0x00u8, 0x00u8];
    assert_eq!(
        encoded.as_slice(), expected,
        "<sce:test-vector> at SCXML L25: encode produced {:?}, expected {:?}",
        encoded, expected
    );
    let mut cursor = SceCursor::new(expected);
    let decoded = CodecLengthRefUint16Le::decode(&mut cursor)
        .expect("<sce:test-vector> at SCXML L25: decode failed");
    assert_eq!(
        cursor.remaining(), 0,
        "<sce:test-vector> at SCXML L25: decode left {} bytes unconsumed",
        cursor.remaining()
    );
    assert_eq!(
        decoded.payload_len, 0x0u16,
        "<sce:test-vector> at SCXML L25: field `payload_len` mismatch"
    );
    assert_eq!(
        decoded.payload, b"",
        "<sce:test-vector> at SCXML L25: field `payload` mismatch"
    );
    // Owned-projection round-trip (consumer-requested; acceptance #2):
    // deep-copy the borrowed decode into its owned mirror and assert
    // every field still equals the oracle. Owned `Vec<u8>` / `String`
    // fields compare directly against the `&[u8]` / `&str` literals.
    let owned = CodecLengthRefUint16Le::decode(&mut SceCursor::new(expected))
        .expect("<sce:test-vector> at SCXML L25: decode for into_owned failed")
        .into_owned();
    assert_eq!(
        owned.payload_len, 0x0u16,
        "<sce:test-vector> at SCXML L25: into_owned field `payload_len` mismatch"
    );
    assert_eq!(
        owned.payload, b"",
        "<sce:test-vector> at SCXML L25: into_owned field `payload` mismatch"
    );
}
#[test]
fn test_vector_codec_length_ref_uint16_le_l29() {
    let actual = CodecLengthRefUint16Le {
        payload_len: 0x4u16,
        payload: b"\xaa\xbb\xcc\xdd",
    };
    let encoded = actual.encode_to_vec();
    let expected: &[u8] = &[0x04u8, 0x00u8, 0xaau8, 0xbbu8, 0xccu8, 0xddu8];
    assert_eq!(
        encoded.as_slice(), expected,
        "<sce:test-vector> at SCXML L29: encode produced {:?}, expected {:?}",
        encoded, expected
    );
    let mut cursor = SceCursor::new(expected);
    let decoded = CodecLengthRefUint16Le::decode(&mut cursor)
        .expect("<sce:test-vector> at SCXML L29: decode failed");
    assert_eq!(
        cursor.remaining(), 0,
        "<sce:test-vector> at SCXML L29: decode left {} bytes unconsumed",
        cursor.remaining()
    );
    assert_eq!(
        decoded.payload_len, 0x4u16,
        "<sce:test-vector> at SCXML L29: field `payload_len` mismatch"
    );
    assert_eq!(
        decoded.payload, b"\xaa\xbb\xcc\xdd",
        "<sce:test-vector> at SCXML L29: field `payload` mismatch"
    );
    // Owned-projection round-trip (consumer-requested; acceptance #2):
    // deep-copy the borrowed decode into its owned mirror and assert
    // every field still equals the oracle. Owned `Vec<u8>` / `String`
    // fields compare directly against the `&[u8]` / `&str` literals.
    let owned = CodecLengthRefUint16Le::decode(&mut SceCursor::new(expected))
        .expect("<sce:test-vector> at SCXML L29: decode for into_owned failed")
        .into_owned();
    assert_eq!(
        owned.payload_len, 0x4u16,
        "<sce:test-vector> at SCXML L29: into_owned field `payload_len` mismatch"
    );
    assert_eq!(
        owned.payload, b"\xaa\xbb\xcc\xdd",
        "<sce:test-vector> at SCXML L29: into_owned field `payload` mismatch"
    );
}
#[test]
fn test_vector_codec_length_ref_uint16_le_l33() {
    let actual = CodecLengthRefUint16Le {
        payload_len: 0x100u16,
        payload: b"\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff",
    };
    let encoded = actual.encode_to_vec();
    let expected: &[u8] = &[0x00u8, 0x01u8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8];
    assert_eq!(
        encoded.as_slice(), expected,
        "<sce:test-vector> at SCXML L33: encode produced {:?}, expected {:?}",
        encoded, expected
    );
    let mut cursor = SceCursor::new(expected);
    let decoded = CodecLengthRefUint16Le::decode(&mut cursor)
        .expect("<sce:test-vector> at SCXML L33: decode failed");
    assert_eq!(
        cursor.remaining(), 0,
        "<sce:test-vector> at SCXML L33: decode left {} bytes unconsumed",
        cursor.remaining()
    );
    assert_eq!(
        decoded.payload_len, 0x100u16,
        "<sce:test-vector> at SCXML L33: field `payload_len` mismatch"
    );
    assert_eq!(
        decoded.payload, b"\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff",
        "<sce:test-vector> at SCXML L33: field `payload` mismatch"
    );
    // Owned-projection round-trip (consumer-requested; acceptance #2):
    // deep-copy the borrowed decode into its owned mirror and assert
    // every field still equals the oracle. Owned `Vec<u8>` / `String`
    // fields compare directly against the `&[u8]` / `&str` literals.
    let owned = CodecLengthRefUint16Le::decode(&mut SceCursor::new(expected))
        .expect("<sce:test-vector> at SCXML L33: decode for into_owned failed")
        .into_owned();
    assert_eq!(
        owned.payload_len, 0x100u16,
        "<sce:test-vector> at SCXML L33: into_owned field `payload_len` mismatch"
    );
    assert_eq!(
        owned.payload, b"\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff",
        "<sce:test-vector> at SCXML L33: into_owned field `payload` mismatch"
    );
}
