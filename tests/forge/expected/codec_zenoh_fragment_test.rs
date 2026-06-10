// SCE Forge: Auto-generated codec test-vector sidecar (RFC §5.B)
// Companion to codec_zenoh_fragment.rs — do not edit; regenerate from the source SCXML.

#[test]
fn test_vector_codec_zenoh_fragment_l35() {
    let actual = CodecZenohFragment {
        sn: 0x0u64,
        payload: b"",
    };
    let encoded = actual.encode_to_vec();
    let expected: &[u8] = &[0x00u8];
    assert_eq!(
        encoded.as_slice(), expected,
        "<sce:test-vector> at SCXML L35: encode produced {:?}, expected {:?}",
        encoded, expected
    );
    let mut cursor = SceCursor::new(expected);
    let decoded = CodecZenohFragment::decode(&mut cursor)
        .expect("<sce:test-vector> at SCXML L35: decode failed");
    assert_eq!(
        cursor.remaining(), 0,
        "<sce:test-vector> at SCXML L35: decode left {} bytes unconsumed",
        cursor.remaining()
    );
    assert_eq!(
        decoded.sn, 0x0u64,
        "<sce:test-vector> at SCXML L35: field `sn` mismatch"
    );
    assert_eq!(
        decoded.payload, b"",
        "<sce:test-vector> at SCXML L35: field `payload` mismatch"
    );
    // Owned-projection round-trip (acceptance #2): deep-copy the borrowed
    // decode into its no-alloc owned mirror and assert every field still
    // equals the oracle. Owned `heapless::Vec<u8, N>` / `heapless::String<N>`
    // fields compare directly against the `&[u8]` / `&str` literals (both
    // deref to the slice / str). `try_into_owned` is fallible (the bounded
    // copy re-checks the decode bound); the decode above already proved the
    // value fits, so the projection cannot fail here.
    let owned = CodecZenohFragment::decode(&mut SceCursor::new(expected))
        .expect("<sce:test-vector> at SCXML L35: decode for into_owned failed")
        .try_into_owned()
        .expect("<sce:test-vector> at SCXML L35: try_into_owned exceeded a bounded field");
    assert_eq!(
        owned.sn, 0x0u64,
        "<sce:test-vector> at SCXML L35: into_owned field `sn` mismatch"
    );
    assert_eq!(
        owned.payload, b"",
        "<sce:test-vector> at SCXML L35: into_owned field `payload` mismatch"
    );
    // Owned→borrowed projection round-trip (acceptance #2): re-borrow the
    // owned mirror back into the zero-copy view and assert it re-encodes to
    // the exact oracle bytes — closing the borrowed→owned→borrowed loop so
    // an owned value reaches the borrowed-only `encode`. Every sidecar-
    // eligible codec is scalar / bytes / string (no bounded list reaches
    // this gate), so the projection is infallible.
    assert_eq!(
        owned.as_borrowed().encode_to_vec().as_slice(), expected,
        "<sce:test-vector> at SCXML L35: as_borrowed re-encode mismatch"
    );
}
#[test]
fn test_vector_codec_zenoh_fragment_l39() {
    let actual = CodecZenohFragment {
        sn: 0x1u64,
        payload: b"\xca\xfe",
    };
    let encoded = actual.encode_to_vec();
    let expected: &[u8] = &[0x01u8, 0xcau8, 0xfeu8];
    assert_eq!(
        encoded.as_slice(), expected,
        "<sce:test-vector> at SCXML L39: encode produced {:?}, expected {:?}",
        encoded, expected
    );
    let mut cursor = SceCursor::new(expected);
    let decoded = CodecZenohFragment::decode(&mut cursor)
        .expect("<sce:test-vector> at SCXML L39: decode failed");
    assert_eq!(
        cursor.remaining(), 0,
        "<sce:test-vector> at SCXML L39: decode left {} bytes unconsumed",
        cursor.remaining()
    );
    assert_eq!(
        decoded.sn, 0x1u64,
        "<sce:test-vector> at SCXML L39: field `sn` mismatch"
    );
    assert_eq!(
        decoded.payload, b"\xca\xfe",
        "<sce:test-vector> at SCXML L39: field `payload` mismatch"
    );
    // Owned-projection round-trip (acceptance #2): deep-copy the borrowed
    // decode into its no-alloc owned mirror and assert every field still
    // equals the oracle. Owned `heapless::Vec<u8, N>` / `heapless::String<N>`
    // fields compare directly against the `&[u8]` / `&str` literals (both
    // deref to the slice / str). `try_into_owned` is fallible (the bounded
    // copy re-checks the decode bound); the decode above already proved the
    // value fits, so the projection cannot fail here.
    let owned = CodecZenohFragment::decode(&mut SceCursor::new(expected))
        .expect("<sce:test-vector> at SCXML L39: decode for into_owned failed")
        .try_into_owned()
        .expect("<sce:test-vector> at SCXML L39: try_into_owned exceeded a bounded field");
    assert_eq!(
        owned.sn, 0x1u64,
        "<sce:test-vector> at SCXML L39: into_owned field `sn` mismatch"
    );
    assert_eq!(
        owned.payload, b"\xca\xfe",
        "<sce:test-vector> at SCXML L39: into_owned field `payload` mismatch"
    );
    // Owned→borrowed projection round-trip (acceptance #2): re-borrow the
    // owned mirror back into the zero-copy view and assert it re-encodes to
    // the exact oracle bytes — closing the borrowed→owned→borrowed loop so
    // an owned value reaches the borrowed-only `encode`. Every sidecar-
    // eligible codec is scalar / bytes / string (no bounded list reaches
    // this gate), so the projection is infallible.
    assert_eq!(
        owned.as_borrowed().encode_to_vec().as_slice(), expected,
        "<sce:test-vector> at SCXML L39: as_borrowed re-encode mismatch"
    );
}
#[test]
fn test_vector_codec_zenoh_fragment_l43() {
    let actual = CodecZenohFragment {
        sn: 0x7fu64,
        payload: b"\xaa\xbb\xcc",
    };
    let encoded = actual.encode_to_vec();
    let expected: &[u8] = &[0x7fu8, 0xaau8, 0xbbu8, 0xccu8];
    assert_eq!(
        encoded.as_slice(), expected,
        "<sce:test-vector> at SCXML L43: encode produced {:?}, expected {:?}",
        encoded, expected
    );
    let mut cursor = SceCursor::new(expected);
    let decoded = CodecZenohFragment::decode(&mut cursor)
        .expect("<sce:test-vector> at SCXML L43: decode failed");
    assert_eq!(
        cursor.remaining(), 0,
        "<sce:test-vector> at SCXML L43: decode left {} bytes unconsumed",
        cursor.remaining()
    );
    assert_eq!(
        decoded.sn, 0x7fu64,
        "<sce:test-vector> at SCXML L43: field `sn` mismatch"
    );
    assert_eq!(
        decoded.payload, b"\xaa\xbb\xcc",
        "<sce:test-vector> at SCXML L43: field `payload` mismatch"
    );
    // Owned-projection round-trip (acceptance #2): deep-copy the borrowed
    // decode into its no-alloc owned mirror and assert every field still
    // equals the oracle. Owned `heapless::Vec<u8, N>` / `heapless::String<N>`
    // fields compare directly against the `&[u8]` / `&str` literals (both
    // deref to the slice / str). `try_into_owned` is fallible (the bounded
    // copy re-checks the decode bound); the decode above already proved the
    // value fits, so the projection cannot fail here.
    let owned = CodecZenohFragment::decode(&mut SceCursor::new(expected))
        .expect("<sce:test-vector> at SCXML L43: decode for into_owned failed")
        .try_into_owned()
        .expect("<sce:test-vector> at SCXML L43: try_into_owned exceeded a bounded field");
    assert_eq!(
        owned.sn, 0x7fu64,
        "<sce:test-vector> at SCXML L43: into_owned field `sn` mismatch"
    );
    assert_eq!(
        owned.payload, b"\xaa\xbb\xcc",
        "<sce:test-vector> at SCXML L43: into_owned field `payload` mismatch"
    );
    // Owned→borrowed projection round-trip (acceptance #2): re-borrow the
    // owned mirror back into the zero-copy view and assert it re-encodes to
    // the exact oracle bytes — closing the borrowed→owned→borrowed loop so
    // an owned value reaches the borrowed-only `encode`. Every sidecar-
    // eligible codec is scalar / bytes / string (no bounded list reaches
    // this gate), so the projection is infallible.
    assert_eq!(
        owned.as_borrowed().encode_to_vec().as_slice(), expected,
        "<sce:test-vector> at SCXML L43: as_borrowed re-encode mismatch"
    );
}
#[test]
fn test_vector_codec_zenoh_fragment_l47() {
    let actual = CodecZenohFragment {
        sn: 0x80u64,
        payload: b"\xde\xad",
    };
    let encoded = actual.encode_to_vec();
    let expected: &[u8] = &[0x80u8, 0x01u8, 0xdeu8, 0xadu8];
    assert_eq!(
        encoded.as_slice(), expected,
        "<sce:test-vector> at SCXML L47: encode produced {:?}, expected {:?}",
        encoded, expected
    );
    let mut cursor = SceCursor::new(expected);
    let decoded = CodecZenohFragment::decode(&mut cursor)
        .expect("<sce:test-vector> at SCXML L47: decode failed");
    assert_eq!(
        cursor.remaining(), 0,
        "<sce:test-vector> at SCXML L47: decode left {} bytes unconsumed",
        cursor.remaining()
    );
    assert_eq!(
        decoded.sn, 0x80u64,
        "<sce:test-vector> at SCXML L47: field `sn` mismatch"
    );
    assert_eq!(
        decoded.payload, b"\xde\xad",
        "<sce:test-vector> at SCXML L47: field `payload` mismatch"
    );
    // Owned-projection round-trip (acceptance #2): deep-copy the borrowed
    // decode into its no-alloc owned mirror and assert every field still
    // equals the oracle. Owned `heapless::Vec<u8, N>` / `heapless::String<N>`
    // fields compare directly against the `&[u8]` / `&str` literals (both
    // deref to the slice / str). `try_into_owned` is fallible (the bounded
    // copy re-checks the decode bound); the decode above already proved the
    // value fits, so the projection cannot fail here.
    let owned = CodecZenohFragment::decode(&mut SceCursor::new(expected))
        .expect("<sce:test-vector> at SCXML L47: decode for into_owned failed")
        .try_into_owned()
        .expect("<sce:test-vector> at SCXML L47: try_into_owned exceeded a bounded field");
    assert_eq!(
        owned.sn, 0x80u64,
        "<sce:test-vector> at SCXML L47: into_owned field `sn` mismatch"
    );
    assert_eq!(
        owned.payload, b"\xde\xad",
        "<sce:test-vector> at SCXML L47: into_owned field `payload` mismatch"
    );
    // Owned→borrowed projection round-trip (acceptance #2): re-borrow the
    // owned mirror back into the zero-copy view and assert it re-encodes to
    // the exact oracle bytes — closing the borrowed→owned→borrowed loop so
    // an owned value reaches the borrowed-only `encode`. Every sidecar-
    // eligible codec is scalar / bytes / string (no bounded list reaches
    // this gate), so the projection is infallible.
    assert_eq!(
        owned.as_borrowed().encode_to_vec().as_slice(), expected,
        "<sce:test-vector> at SCXML L47: as_borrowed re-encode mismatch"
    );
}
