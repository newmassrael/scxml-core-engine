// SCE Forge: Auto-generated codec test-vector sidecar (RFC §synth-5-B)
// Companion to codec_fixed_after_lengthref.rs — do not edit; regenerate from the source SCXML.

#[test]
fn test_vector_codec_fixed_after_lengthref_l30() {
    let actual = CodecFixedAfterLengthref {
        header: 0xaau8,
        payload_len: 0x3u16,
        payload: b"\xde\xad\xbe",
        crc32: 0x11223344u32,
    };
    let encoded = actual.encode_to_vec();
    let expected: &[u8] = &[0xaau8, 0x03u8, 0x00u8, 0xdeu8, 0xadu8, 0xbeu8, 0x44u8, 0x33u8, 0x22u8, 0x11u8];
    assert_eq!(
        encoded.as_slice(), expected,
        "<sce:test-vector> at SCXML L30: encode produced {:?}, expected {:?}",
        encoded, expected
    );
    let mut cursor = SceCursor::new(expected);
    let decoded = CodecFixedAfterLengthref::decode(&mut cursor)
        .expect("<sce:test-vector> at SCXML L30: decode failed");
    assert_eq!(
        cursor.remaining(), 0,
        "<sce:test-vector> at SCXML L30: decode left {} bytes unconsumed",
        cursor.remaining()
    );
    assert_eq!(
        decoded.header, 0xaau8,
        "<sce:test-vector> at SCXML L30: field `header` mismatch"
    );
    assert_eq!(
        decoded.payload_len, 0x3u16,
        "<sce:test-vector> at SCXML L30: field `payload_len` mismatch"
    );
    assert_eq!(
        decoded.payload, b"\xde\xad\xbe",
        "<sce:test-vector> at SCXML L30: field `payload` mismatch"
    );
    assert_eq!(
        decoded.crc32, 0x11223344u32,
        "<sce:test-vector> at SCXML L30: field `crc32` mismatch"
    );
    // Owned-projection round-trip (acceptance #2): deep-copy the borrowed
    // decode into its no-alloc owned mirror and assert every field still
    // equals the oracle. Owned `heapless::Vec<u8, N>` / `heapless::String<N>`
    // fields compare directly against the `&[u8]` / `&str` literals (both
    // deref to the slice / str). `try_into_owned` is fallible (the bounded
    // copy re-checks the decode bound); the decode above already proved the
    // value fits, so the projection cannot fail here.
    let owned = CodecFixedAfterLengthref::decode(&mut SceCursor::new(expected))
        .expect("<sce:test-vector> at SCXML L30: decode for into_owned failed")
        .try_into_owned()
        .expect("<sce:test-vector> at SCXML L30: try_into_owned exceeded a bounded field");
    assert_eq!(
        owned.header, 0xaau8,
        "<sce:test-vector> at SCXML L30: into_owned field `header` mismatch"
    );
    assert_eq!(
        owned.payload_len, 0x3u16,
        "<sce:test-vector> at SCXML L30: into_owned field `payload_len` mismatch"
    );
    assert_eq!(
        owned.payload, b"\xde\xad\xbe",
        "<sce:test-vector> at SCXML L30: into_owned field `payload` mismatch"
    );
    assert_eq!(
        owned.crc32, 0x11223344u32,
        "<sce:test-vector> at SCXML L30: into_owned field `crc32` mismatch"
    );
    // Owned→borrowed projection round-trip (acceptance #2): re-borrow the
    // owned mirror back into the zero-copy view and assert it re-encodes to
    // the exact oracle bytes — closing the borrowed→owned→borrowed loop so
    // an owned value reaches the borrowed-only `encode`. Every sidecar-
    // eligible codec is scalar / bytes / string (no bounded list reaches
    // this gate), so the projection is infallible.
    assert_eq!(
        owned.as_borrowed().encode_to_vec().as_slice(), expected,
        "<sce:test-vector> at SCXML L30: as_borrowed re-encode mismatch"
    );
}
#[test]
fn test_vector_codec_fixed_after_lengthref_l37() {
    let actual = CodecFixedAfterLengthref {
        header: 0x1u8,
        payload_len: 0x0u16,
        payload: b"",
        crc32: 0xcafebabeu32,
    };
    let encoded = actual.encode_to_vec();
    let expected: &[u8] = &[0x01u8, 0x00u8, 0x00u8, 0xbeu8, 0xbau8, 0xfeu8, 0xcau8];
    assert_eq!(
        encoded.as_slice(), expected,
        "<sce:test-vector> at SCXML L37: encode produced {:?}, expected {:?}",
        encoded, expected
    );
    let mut cursor = SceCursor::new(expected);
    let decoded = CodecFixedAfterLengthref::decode(&mut cursor)
        .expect("<sce:test-vector> at SCXML L37: decode failed");
    assert_eq!(
        cursor.remaining(), 0,
        "<sce:test-vector> at SCXML L37: decode left {} bytes unconsumed",
        cursor.remaining()
    );
    assert_eq!(
        decoded.header, 0x1u8,
        "<sce:test-vector> at SCXML L37: field `header` mismatch"
    );
    assert_eq!(
        decoded.payload_len, 0x0u16,
        "<sce:test-vector> at SCXML L37: field `payload_len` mismatch"
    );
    assert_eq!(
        decoded.payload, b"",
        "<sce:test-vector> at SCXML L37: field `payload` mismatch"
    );
    assert_eq!(
        decoded.crc32, 0xcafebabeu32,
        "<sce:test-vector> at SCXML L37: field `crc32` mismatch"
    );
    // Owned-projection round-trip (acceptance #2): deep-copy the borrowed
    // decode into its no-alloc owned mirror and assert every field still
    // equals the oracle. Owned `heapless::Vec<u8, N>` / `heapless::String<N>`
    // fields compare directly against the `&[u8]` / `&str` literals (both
    // deref to the slice / str). `try_into_owned` is fallible (the bounded
    // copy re-checks the decode bound); the decode above already proved the
    // value fits, so the projection cannot fail here.
    let owned = CodecFixedAfterLengthref::decode(&mut SceCursor::new(expected))
        .expect("<sce:test-vector> at SCXML L37: decode for into_owned failed")
        .try_into_owned()
        .expect("<sce:test-vector> at SCXML L37: try_into_owned exceeded a bounded field");
    assert_eq!(
        owned.header, 0x1u8,
        "<sce:test-vector> at SCXML L37: into_owned field `header` mismatch"
    );
    assert_eq!(
        owned.payload_len, 0x0u16,
        "<sce:test-vector> at SCXML L37: into_owned field `payload_len` mismatch"
    );
    assert_eq!(
        owned.payload, b"",
        "<sce:test-vector> at SCXML L37: into_owned field `payload` mismatch"
    );
    assert_eq!(
        owned.crc32, 0xcafebabeu32,
        "<sce:test-vector> at SCXML L37: into_owned field `crc32` mismatch"
    );
    // Owned→borrowed projection round-trip (acceptance #2): re-borrow the
    // owned mirror back into the zero-copy view and assert it re-encodes to
    // the exact oracle bytes — closing the borrowed→owned→borrowed loop so
    // an owned value reaches the borrowed-only `encode`. Every sidecar-
    // eligible codec is scalar / bytes / string (no bounded list reaches
    // this gate), so the projection is infallible.
    assert_eq!(
        owned.as_borrowed().encode_to_vec().as_slice(), expected,
        "<sce:test-vector> at SCXML L37: as_borrowed re-encode mismatch"
    );
}
#[test]
fn test_vector_codec_fixed_after_lengthref_l44() {
    let actual = CodecFixedAfterLengthref {
        header: 0xffu8,
        payload_len: 0x5u16,
        payload: b"\x01\x02\x03\x04\x05",
        crc32: 0x1u32,
    };
    let encoded = actual.encode_to_vec();
    let expected: &[u8] = &[0xffu8, 0x05u8, 0x00u8, 0x01u8, 0x02u8, 0x03u8, 0x04u8, 0x05u8, 0x01u8, 0x00u8, 0x00u8, 0x00u8];
    assert_eq!(
        encoded.as_slice(), expected,
        "<sce:test-vector> at SCXML L44: encode produced {:?}, expected {:?}",
        encoded, expected
    );
    let mut cursor = SceCursor::new(expected);
    let decoded = CodecFixedAfterLengthref::decode(&mut cursor)
        .expect("<sce:test-vector> at SCXML L44: decode failed");
    assert_eq!(
        cursor.remaining(), 0,
        "<sce:test-vector> at SCXML L44: decode left {} bytes unconsumed",
        cursor.remaining()
    );
    assert_eq!(
        decoded.header, 0xffu8,
        "<sce:test-vector> at SCXML L44: field `header` mismatch"
    );
    assert_eq!(
        decoded.payload_len, 0x5u16,
        "<sce:test-vector> at SCXML L44: field `payload_len` mismatch"
    );
    assert_eq!(
        decoded.payload, b"\x01\x02\x03\x04\x05",
        "<sce:test-vector> at SCXML L44: field `payload` mismatch"
    );
    assert_eq!(
        decoded.crc32, 0x1u32,
        "<sce:test-vector> at SCXML L44: field `crc32` mismatch"
    );
    // Owned-projection round-trip (acceptance #2): deep-copy the borrowed
    // decode into its no-alloc owned mirror and assert every field still
    // equals the oracle. Owned `heapless::Vec<u8, N>` / `heapless::String<N>`
    // fields compare directly against the `&[u8]` / `&str` literals (both
    // deref to the slice / str). `try_into_owned` is fallible (the bounded
    // copy re-checks the decode bound); the decode above already proved the
    // value fits, so the projection cannot fail here.
    let owned = CodecFixedAfterLengthref::decode(&mut SceCursor::new(expected))
        .expect("<sce:test-vector> at SCXML L44: decode for into_owned failed")
        .try_into_owned()
        .expect("<sce:test-vector> at SCXML L44: try_into_owned exceeded a bounded field");
    assert_eq!(
        owned.header, 0xffu8,
        "<sce:test-vector> at SCXML L44: into_owned field `header` mismatch"
    );
    assert_eq!(
        owned.payload_len, 0x5u16,
        "<sce:test-vector> at SCXML L44: into_owned field `payload_len` mismatch"
    );
    assert_eq!(
        owned.payload, b"\x01\x02\x03\x04\x05",
        "<sce:test-vector> at SCXML L44: into_owned field `payload` mismatch"
    );
    assert_eq!(
        owned.crc32, 0x1u32,
        "<sce:test-vector> at SCXML L44: into_owned field `crc32` mismatch"
    );
    // Owned→borrowed projection round-trip (acceptance #2): re-borrow the
    // owned mirror back into the zero-copy view and assert it re-encodes to
    // the exact oracle bytes — closing the borrowed→owned→borrowed loop so
    // an owned value reaches the borrowed-only `encode`. Every sidecar-
    // eligible codec is scalar / bytes / string (no bounded list reaches
    // this gate), so the projection is infallible.
    assert_eq!(
        owned.as_borrowed().encode_to_vec().as_slice(), expected,
        "<sce:test-vector> at SCXML L44: as_borrowed re-encode mismatch"
    );
}
