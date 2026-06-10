// SCE Forge: Auto-generated codec test-vector sidecar (RFC §5.B)
// Companion to codec_length_ref_uint16_be.rs — do not edit; regenerate from the source SCXML.

#[test]
fn test_vector_codec_length_ref_uint16_be_l20() {
    let actual = CodecLengthRefUint16Be {
        payload_len: 0x0u16,
        payload: b"",
    };
    let encoded = actual.encode_to_vec();
    let expected: &[u8] = &[0x00u8, 0x00u8];
    assert_eq!(
        encoded.as_slice(), expected,
        "<sce:test-vector> at SCXML L20: encode produced {:?}, expected {:?}",
        encoded, expected
    );
    let mut cursor = SceCursor::new(expected);
    let decoded = CodecLengthRefUint16Be::decode(&mut cursor)
        .expect("<sce:test-vector> at SCXML L20: decode failed");
    assert_eq!(
        cursor.remaining(), 0,
        "<sce:test-vector> at SCXML L20: decode left {} bytes unconsumed",
        cursor.remaining()
    );
    assert_eq!(
        decoded.payload_len, 0x0u16,
        "<sce:test-vector> at SCXML L20: field `payload_len` mismatch"
    );
    assert_eq!(
        decoded.payload, b"",
        "<sce:test-vector> at SCXML L20: field `payload` mismatch"
    );
    // Owned-projection round-trip (acceptance #2): deep-copy the borrowed
    // decode into its no-alloc owned mirror and assert every field still
    // equals the oracle. Owned `heapless::Vec<u8, N>` / `heapless::String<N>`
    // fields compare directly against the `&[u8]` / `&str` literals (both
    // deref to the slice / str). `try_into_owned` is fallible (the bounded
    // copy re-checks the decode bound); the decode above already proved the
    // value fits, so the projection cannot fail here.
    let owned = CodecLengthRefUint16Be::decode(&mut SceCursor::new(expected))
        .expect("<sce:test-vector> at SCXML L20: decode for into_owned failed")
        .try_into_owned()
        .expect("<sce:test-vector> at SCXML L20: try_into_owned exceeded a bounded field");
    assert_eq!(
        owned.payload_len, 0x0u16,
        "<sce:test-vector> at SCXML L20: into_owned field `payload_len` mismatch"
    );
    assert_eq!(
        owned.payload, b"",
        "<sce:test-vector> at SCXML L20: into_owned field `payload` mismatch"
    );
    // Owned→borrowed projection round-trip (acceptance #2): re-borrow the
    // owned mirror back into the zero-copy view and assert it re-encodes to
    // the exact oracle bytes — closing the borrowed→owned→borrowed loop so
    // an owned value reaches the borrowed-only `encode`. Every sidecar-
    // eligible codec is scalar / bytes / string (no bounded list reaches
    // this gate), so the projection is infallible.
    assert_eq!(
        owned.as_borrowed().encode_to_vec().as_slice(), expected,
        "<sce:test-vector> at SCXML L20: as_borrowed re-encode mismatch"
    );
}
#[test]
fn test_vector_codec_length_ref_uint16_be_l24() {
    let actual = CodecLengthRefUint16Be {
        payload_len: 0x4u16,
        payload: b"\xaa\xbb\xcc\xdd",
    };
    let encoded = actual.encode_to_vec();
    let expected: &[u8] = &[0x00u8, 0x04u8, 0xaau8, 0xbbu8, 0xccu8, 0xddu8];
    assert_eq!(
        encoded.as_slice(), expected,
        "<sce:test-vector> at SCXML L24: encode produced {:?}, expected {:?}",
        encoded, expected
    );
    let mut cursor = SceCursor::new(expected);
    let decoded = CodecLengthRefUint16Be::decode(&mut cursor)
        .expect("<sce:test-vector> at SCXML L24: decode failed");
    assert_eq!(
        cursor.remaining(), 0,
        "<sce:test-vector> at SCXML L24: decode left {} bytes unconsumed",
        cursor.remaining()
    );
    assert_eq!(
        decoded.payload_len, 0x4u16,
        "<sce:test-vector> at SCXML L24: field `payload_len` mismatch"
    );
    assert_eq!(
        decoded.payload, b"\xaa\xbb\xcc\xdd",
        "<sce:test-vector> at SCXML L24: field `payload` mismatch"
    );
    // Owned-projection round-trip (acceptance #2): deep-copy the borrowed
    // decode into its no-alloc owned mirror and assert every field still
    // equals the oracle. Owned `heapless::Vec<u8, N>` / `heapless::String<N>`
    // fields compare directly against the `&[u8]` / `&str` literals (both
    // deref to the slice / str). `try_into_owned` is fallible (the bounded
    // copy re-checks the decode bound); the decode above already proved the
    // value fits, so the projection cannot fail here.
    let owned = CodecLengthRefUint16Be::decode(&mut SceCursor::new(expected))
        .expect("<sce:test-vector> at SCXML L24: decode for into_owned failed")
        .try_into_owned()
        .expect("<sce:test-vector> at SCXML L24: try_into_owned exceeded a bounded field");
    assert_eq!(
        owned.payload_len, 0x4u16,
        "<sce:test-vector> at SCXML L24: into_owned field `payload_len` mismatch"
    );
    assert_eq!(
        owned.payload, b"\xaa\xbb\xcc\xdd",
        "<sce:test-vector> at SCXML L24: into_owned field `payload` mismatch"
    );
    // Owned→borrowed projection round-trip (acceptance #2): re-borrow the
    // owned mirror back into the zero-copy view and assert it re-encodes to
    // the exact oracle bytes — closing the borrowed→owned→borrowed loop so
    // an owned value reaches the borrowed-only `encode`. Every sidecar-
    // eligible codec is scalar / bytes / string (no bounded list reaches
    // this gate), so the projection is infallible.
    assert_eq!(
        owned.as_borrowed().encode_to_vec().as_slice(), expected,
        "<sce:test-vector> at SCXML L24: as_borrowed re-encode mismatch"
    );
}
#[test]
fn test_vector_codec_length_ref_uint16_be_l28() {
    let actual = CodecLengthRefUint16Be {
        payload_len: 0x100u16,
        payload: b"\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff",
    };
    let encoded = actual.encode_to_vec();
    let expected: &[u8] = &[0x01u8, 0x00u8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8];
    assert_eq!(
        encoded.as_slice(), expected,
        "<sce:test-vector> at SCXML L28: encode produced {:?}, expected {:?}",
        encoded, expected
    );
    let mut cursor = SceCursor::new(expected);
    let decoded = CodecLengthRefUint16Be::decode(&mut cursor)
        .expect("<sce:test-vector> at SCXML L28: decode failed");
    assert_eq!(
        cursor.remaining(), 0,
        "<sce:test-vector> at SCXML L28: decode left {} bytes unconsumed",
        cursor.remaining()
    );
    assert_eq!(
        decoded.payload_len, 0x100u16,
        "<sce:test-vector> at SCXML L28: field `payload_len` mismatch"
    );
    assert_eq!(
        decoded.payload, b"\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff",
        "<sce:test-vector> at SCXML L28: field `payload` mismatch"
    );
    // Owned-projection round-trip (acceptance #2): deep-copy the borrowed
    // decode into its no-alloc owned mirror and assert every field still
    // equals the oracle. Owned `heapless::Vec<u8, N>` / `heapless::String<N>`
    // fields compare directly against the `&[u8]` / `&str` literals (both
    // deref to the slice / str). `try_into_owned` is fallible (the bounded
    // copy re-checks the decode bound); the decode above already proved the
    // value fits, so the projection cannot fail here.
    let owned = CodecLengthRefUint16Be::decode(&mut SceCursor::new(expected))
        .expect("<sce:test-vector> at SCXML L28: decode for into_owned failed")
        .try_into_owned()
        .expect("<sce:test-vector> at SCXML L28: try_into_owned exceeded a bounded field");
    assert_eq!(
        owned.payload_len, 0x100u16,
        "<sce:test-vector> at SCXML L28: into_owned field `payload_len` mismatch"
    );
    assert_eq!(
        owned.payload, b"\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff",
        "<sce:test-vector> at SCXML L28: into_owned field `payload` mismatch"
    );
    // Owned→borrowed projection round-trip (acceptance #2): re-borrow the
    // owned mirror back into the zero-copy view and assert it re-encodes to
    // the exact oracle bytes — closing the borrowed→owned→borrowed loop so
    // an owned value reaches the borrowed-only `encode`. Every sidecar-
    // eligible codec is scalar / bytes / string (no bounded list reaches
    // this gate), so the projection is infallible.
    assert_eq!(
        owned.as_borrowed().encode_to_vec().as_slice(), expected,
        "<sce:test-vector> at SCXML L28: as_borrowed re-encode mismatch"
    );
}
