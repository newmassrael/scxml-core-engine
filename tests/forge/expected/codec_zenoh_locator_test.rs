// SCE Forge: Auto-generated codec test-vector sidecar (RFC §synth-5-B)
// Companion to codec_zenoh_locator.rs — do not edit; regenerate from the source SCXML.

#[test]
fn test_vector_codec_zenoh_locator_l38() {
    let actual = CodecZenohLocator {
        locator_len: 0x0u64,
        locator: "",
    };
    let encoded = actual.encode_to_vec();
    let expected: &[u8] = &[0x00u8];
    assert_eq!(
        encoded.as_slice(), expected,
        "<sce:test-vector> at SCXML L38: encode produced {:?}, expected {:?}",
        encoded, expected
    );
    let mut cursor = SceCursor::new(expected);
    let decoded = CodecZenohLocator::decode(&mut cursor)
        .expect("<sce:test-vector> at SCXML L38: decode failed");
    assert_eq!(
        cursor.remaining(), 0,
        "<sce:test-vector> at SCXML L38: decode left {} bytes unconsumed",
        cursor.remaining()
    );
    assert_eq!(
        decoded.locator_len, 0x0u64,
        "<sce:test-vector> at SCXML L38: field `locator_len` mismatch"
    );
    assert_eq!(
        decoded.locator, "",
        "<sce:test-vector> at SCXML L38: field `locator` mismatch"
    );
    // Owned-projection round-trip (acceptance #2): deep-copy the borrowed
    // decode into its no-alloc owned mirror and assert every field still
    // equals the oracle. Owned `heapless::Vec<u8, N>` / `heapless::String<N>`
    // fields compare directly against the `&[u8]` / `&str` literals (both
    // deref to the slice / str). `try_into_owned` is fallible (the bounded
    // copy re-checks the decode bound); the decode above already proved the
    // value fits, so the projection cannot fail here.
    let owned = CodecZenohLocator::decode(&mut SceCursor::new(expected))
        .expect("<sce:test-vector> at SCXML L38: decode for into_owned failed")
        .try_into_owned()
        .expect("<sce:test-vector> at SCXML L38: try_into_owned exceeded a bounded field");
    assert_eq!(
        owned.locator_len, 0x0u64,
        "<sce:test-vector> at SCXML L38: into_owned field `locator_len` mismatch"
    );
    assert_eq!(
        owned.locator, "",
        "<sce:test-vector> at SCXML L38: into_owned field `locator` mismatch"
    );
    // Owned→borrowed projection round-trip (acceptance #2): re-borrow the
    // owned mirror back into the zero-copy view and assert it re-encodes to
    // the exact oracle bytes — closing the borrowed→owned→borrowed loop so
    // an owned value reaches the borrowed-only `encode`. Every sidecar-
    // eligible codec is scalar / bytes / string (no bounded list reaches
    // this gate), so the projection is infallible.
    assert_eq!(
        owned.as_borrowed().encode_to_vec().as_slice(), expected,
        "<sce:test-vector> at SCXML L38: as_borrowed re-encode mismatch"
    );
}
#[test]
fn test_vector_codec_zenoh_locator_l42() {
    let actual = CodecZenohLocator {
        locator_len: 0x3u64,
        locator: "abc",
    };
    let encoded = actual.encode_to_vec();
    let expected: &[u8] = &[0x03u8, 0x61u8, 0x62u8, 0x63u8];
    assert_eq!(
        encoded.as_slice(), expected,
        "<sce:test-vector> at SCXML L42: encode produced {:?}, expected {:?}",
        encoded, expected
    );
    let mut cursor = SceCursor::new(expected);
    let decoded = CodecZenohLocator::decode(&mut cursor)
        .expect("<sce:test-vector> at SCXML L42: decode failed");
    assert_eq!(
        cursor.remaining(), 0,
        "<sce:test-vector> at SCXML L42: decode left {} bytes unconsumed",
        cursor.remaining()
    );
    assert_eq!(
        decoded.locator_len, 0x3u64,
        "<sce:test-vector> at SCXML L42: field `locator_len` mismatch"
    );
    assert_eq!(
        decoded.locator, "abc",
        "<sce:test-vector> at SCXML L42: field `locator` mismatch"
    );
    // Owned-projection round-trip (acceptance #2): deep-copy the borrowed
    // decode into its no-alloc owned mirror and assert every field still
    // equals the oracle. Owned `heapless::Vec<u8, N>` / `heapless::String<N>`
    // fields compare directly against the `&[u8]` / `&str` literals (both
    // deref to the slice / str). `try_into_owned` is fallible (the bounded
    // copy re-checks the decode bound); the decode above already proved the
    // value fits, so the projection cannot fail here.
    let owned = CodecZenohLocator::decode(&mut SceCursor::new(expected))
        .expect("<sce:test-vector> at SCXML L42: decode for into_owned failed")
        .try_into_owned()
        .expect("<sce:test-vector> at SCXML L42: try_into_owned exceeded a bounded field");
    assert_eq!(
        owned.locator_len, 0x3u64,
        "<sce:test-vector> at SCXML L42: into_owned field `locator_len` mismatch"
    );
    assert_eq!(
        owned.locator, "abc",
        "<sce:test-vector> at SCXML L42: into_owned field `locator` mismatch"
    );
    // Owned→borrowed projection round-trip (acceptance #2): re-borrow the
    // owned mirror back into the zero-copy view and assert it re-encodes to
    // the exact oracle bytes — closing the borrowed→owned→borrowed loop so
    // an owned value reaches the borrowed-only `encode`. Every sidecar-
    // eligible codec is scalar / bytes / string (no bounded list reaches
    // this gate), so the projection is infallible.
    assert_eq!(
        owned.as_borrowed().encode_to_vec().as_slice(), expected,
        "<sce:test-vector> at SCXML L42: as_borrowed re-encode mismatch"
    );
}
#[test]
fn test_vector_codec_zenoh_locator_l46() {
    let actual = CodecZenohLocator {
        locator_len: 0x12u64,
        locator: "tcp/127.0.0.1:7447",
    };
    let encoded = actual.encode_to_vec();
    let expected: &[u8] = &[0x12u8, 0x74u8, 0x63u8, 0x70u8, 0x2fu8, 0x31u8, 0x32u8, 0x37u8, 0x2eu8, 0x30u8, 0x2eu8, 0x30u8, 0x2eu8, 0x31u8, 0x3au8, 0x37u8, 0x34u8, 0x34u8, 0x37u8];
    assert_eq!(
        encoded.as_slice(), expected,
        "<sce:test-vector> at SCXML L46: encode produced {:?}, expected {:?}",
        encoded, expected
    );
    let mut cursor = SceCursor::new(expected);
    let decoded = CodecZenohLocator::decode(&mut cursor)
        .expect("<sce:test-vector> at SCXML L46: decode failed");
    assert_eq!(
        cursor.remaining(), 0,
        "<sce:test-vector> at SCXML L46: decode left {} bytes unconsumed",
        cursor.remaining()
    );
    assert_eq!(
        decoded.locator_len, 0x12u64,
        "<sce:test-vector> at SCXML L46: field `locator_len` mismatch"
    );
    assert_eq!(
        decoded.locator, "tcp/127.0.0.1:7447",
        "<sce:test-vector> at SCXML L46: field `locator` mismatch"
    );
    // Owned-projection round-trip (acceptance #2): deep-copy the borrowed
    // decode into its no-alloc owned mirror and assert every field still
    // equals the oracle. Owned `heapless::Vec<u8, N>` / `heapless::String<N>`
    // fields compare directly against the `&[u8]` / `&str` literals (both
    // deref to the slice / str). `try_into_owned` is fallible (the bounded
    // copy re-checks the decode bound); the decode above already proved the
    // value fits, so the projection cannot fail here.
    let owned = CodecZenohLocator::decode(&mut SceCursor::new(expected))
        .expect("<sce:test-vector> at SCXML L46: decode for into_owned failed")
        .try_into_owned()
        .expect("<sce:test-vector> at SCXML L46: try_into_owned exceeded a bounded field");
    assert_eq!(
        owned.locator_len, 0x12u64,
        "<sce:test-vector> at SCXML L46: into_owned field `locator_len` mismatch"
    );
    assert_eq!(
        owned.locator, "tcp/127.0.0.1:7447",
        "<sce:test-vector> at SCXML L46: into_owned field `locator` mismatch"
    );
    // Owned→borrowed projection round-trip (acceptance #2): re-borrow the
    // owned mirror back into the zero-copy view and assert it re-encodes to
    // the exact oracle bytes — closing the borrowed→owned→borrowed loop so
    // an owned value reaches the borrowed-only `encode`. Every sidecar-
    // eligible codec is scalar / bytes / string (no bounded list reaches
    // this gate), so the projection is infallible.
    assert_eq!(
        owned.as_borrowed().encode_to_vec().as_slice(), expected,
        "<sce:test-vector> at SCXML L46: as_borrowed re-encode mismatch"
    );
}
#[test]
fn test_vector_codec_zenoh_locator_l50() {
    let actual = CodecZenohLocator {
        locator_len: 0x6u64,
        locator: "héllo",
    };
    let encoded = actual.encode_to_vec();
    let expected: &[u8] = &[0x06u8, 0x68u8, 0xc3u8, 0xa9u8, 0x6cu8, 0x6cu8, 0x6fu8];
    assert_eq!(
        encoded.as_slice(), expected,
        "<sce:test-vector> at SCXML L50: encode produced {:?}, expected {:?}",
        encoded, expected
    );
    let mut cursor = SceCursor::new(expected);
    let decoded = CodecZenohLocator::decode(&mut cursor)
        .expect("<sce:test-vector> at SCXML L50: decode failed");
    assert_eq!(
        cursor.remaining(), 0,
        "<sce:test-vector> at SCXML L50: decode left {} bytes unconsumed",
        cursor.remaining()
    );
    assert_eq!(
        decoded.locator_len, 0x6u64,
        "<sce:test-vector> at SCXML L50: field `locator_len` mismatch"
    );
    assert_eq!(
        decoded.locator, "héllo",
        "<sce:test-vector> at SCXML L50: field `locator` mismatch"
    );
    // Owned-projection round-trip (acceptance #2): deep-copy the borrowed
    // decode into its no-alloc owned mirror and assert every field still
    // equals the oracle. Owned `heapless::Vec<u8, N>` / `heapless::String<N>`
    // fields compare directly against the `&[u8]` / `&str` literals (both
    // deref to the slice / str). `try_into_owned` is fallible (the bounded
    // copy re-checks the decode bound); the decode above already proved the
    // value fits, so the projection cannot fail here.
    let owned = CodecZenohLocator::decode(&mut SceCursor::new(expected))
        .expect("<sce:test-vector> at SCXML L50: decode for into_owned failed")
        .try_into_owned()
        .expect("<sce:test-vector> at SCXML L50: try_into_owned exceeded a bounded field");
    assert_eq!(
        owned.locator_len, 0x6u64,
        "<sce:test-vector> at SCXML L50: into_owned field `locator_len` mismatch"
    );
    assert_eq!(
        owned.locator, "héllo",
        "<sce:test-vector> at SCXML L50: into_owned field `locator` mismatch"
    );
    // Owned→borrowed projection round-trip (acceptance #2): re-borrow the
    // owned mirror back into the zero-copy view and assert it re-encodes to
    // the exact oracle bytes — closing the borrowed→owned→borrowed loop so
    // an owned value reaches the borrowed-only `encode`. Every sidecar-
    // eligible codec is scalar / bytes / string (no bounded list reaches
    // this gate), so the projection is infallible.
    assert_eq!(
        owned.as_borrowed().encode_to_vec().as_slice(), expected,
        "<sce:test-vector> at SCXML L50: as_borrowed re-encode mismatch"
    );
}
