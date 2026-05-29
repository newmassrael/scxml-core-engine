// SCE Forge: Auto-generated codec test-vector sidecar (RFC §5.B B5-θ)
// Companion to codec_zenoh_frame.rs — do not edit; regenerate from the source SCXML.

#[test]
fn test_vector_codec_zenoh_frame_l38() {
    let actual = CodecZenohFrame {
        sn: 0x0u64,
        payload: b"",
    };
    let encoded = actual.encode_to_vec();
    let expected: &[u8] = &[0x00u8];
    assert_eq!(
        encoded.as_slice(), expected,
        "<sce:test-vector> at SCXML L38: encode produced {:?}, expected {:?}",
        encoded, expected
    );
    let mut cursor = SceCursor::new(expected);
    let decoded = CodecZenohFrame::decode(&mut cursor)
        .expect("<sce:test-vector> at SCXML L38: decode failed");
    assert_eq!(
        cursor.remaining(), 0,
        "<sce:test-vector> at SCXML L38: decode left {} bytes unconsumed",
        cursor.remaining()
    );
    assert_eq!(
        decoded.sn, 0x0u64,
        "<sce:test-vector> at SCXML L38: field `sn` mismatch"
    );
    assert_eq!(
        decoded.payload, b"",
        "<sce:test-vector> at SCXML L38: field `payload` mismatch"
    );
    // Owned-projection round-trip (consumer-requested; acceptance #2):
    // deep-copy the borrowed decode into its owned mirror and assert
    // every field still equals the oracle. Owned `Vec<u8>` / `String`
    // fields compare directly against the `&[u8]` / `&str` literals.
    let owned = CodecZenohFrame::decode(&mut SceCursor::new(expected))
        .expect("<sce:test-vector> at SCXML L38: decode for into_owned failed")
        .into_owned();
    assert_eq!(
        owned.sn, 0x0u64,
        "<sce:test-vector> at SCXML L38: into_owned field `sn` mismatch"
    );
    assert_eq!(
        owned.payload, b"",
        "<sce:test-vector> at SCXML L38: into_owned field `payload` mismatch"
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
fn test_vector_codec_zenoh_frame_l42() {
    let actual = CodecZenohFrame {
        sn: 0x1u64,
        payload: b"\xca\xfe",
    };
    let encoded = actual.encode_to_vec();
    let expected: &[u8] = &[0x01u8, 0xcau8, 0xfeu8];
    assert_eq!(
        encoded.as_slice(), expected,
        "<sce:test-vector> at SCXML L42: encode produced {:?}, expected {:?}",
        encoded, expected
    );
    let mut cursor = SceCursor::new(expected);
    let decoded = CodecZenohFrame::decode(&mut cursor)
        .expect("<sce:test-vector> at SCXML L42: decode failed");
    assert_eq!(
        cursor.remaining(), 0,
        "<sce:test-vector> at SCXML L42: decode left {} bytes unconsumed",
        cursor.remaining()
    );
    assert_eq!(
        decoded.sn, 0x1u64,
        "<sce:test-vector> at SCXML L42: field `sn` mismatch"
    );
    assert_eq!(
        decoded.payload, b"\xca\xfe",
        "<sce:test-vector> at SCXML L42: field `payload` mismatch"
    );
    // Owned-projection round-trip (consumer-requested; acceptance #2):
    // deep-copy the borrowed decode into its owned mirror and assert
    // every field still equals the oracle. Owned `Vec<u8>` / `String`
    // fields compare directly against the `&[u8]` / `&str` literals.
    let owned = CodecZenohFrame::decode(&mut SceCursor::new(expected))
        .expect("<sce:test-vector> at SCXML L42: decode for into_owned failed")
        .into_owned();
    assert_eq!(
        owned.sn, 0x1u64,
        "<sce:test-vector> at SCXML L42: into_owned field `sn` mismatch"
    );
    assert_eq!(
        owned.payload, b"\xca\xfe",
        "<sce:test-vector> at SCXML L42: into_owned field `payload` mismatch"
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
fn test_vector_codec_zenoh_frame_l46() {
    let actual = CodecZenohFrame {
        sn: 0x7fu64,
        payload: b"\xaa\xbb\xcc",
    };
    let encoded = actual.encode_to_vec();
    let expected: &[u8] = &[0x7fu8, 0xaau8, 0xbbu8, 0xccu8];
    assert_eq!(
        encoded.as_slice(), expected,
        "<sce:test-vector> at SCXML L46: encode produced {:?}, expected {:?}",
        encoded, expected
    );
    let mut cursor = SceCursor::new(expected);
    let decoded = CodecZenohFrame::decode(&mut cursor)
        .expect("<sce:test-vector> at SCXML L46: decode failed");
    assert_eq!(
        cursor.remaining(), 0,
        "<sce:test-vector> at SCXML L46: decode left {} bytes unconsumed",
        cursor.remaining()
    );
    assert_eq!(
        decoded.sn, 0x7fu64,
        "<sce:test-vector> at SCXML L46: field `sn` mismatch"
    );
    assert_eq!(
        decoded.payload, b"\xaa\xbb\xcc",
        "<sce:test-vector> at SCXML L46: field `payload` mismatch"
    );
    // Owned-projection round-trip (consumer-requested; acceptance #2):
    // deep-copy the borrowed decode into its owned mirror and assert
    // every field still equals the oracle. Owned `Vec<u8>` / `String`
    // fields compare directly against the `&[u8]` / `&str` literals.
    let owned = CodecZenohFrame::decode(&mut SceCursor::new(expected))
        .expect("<sce:test-vector> at SCXML L46: decode for into_owned failed")
        .into_owned();
    assert_eq!(
        owned.sn, 0x7fu64,
        "<sce:test-vector> at SCXML L46: into_owned field `sn` mismatch"
    );
    assert_eq!(
        owned.payload, b"\xaa\xbb\xcc",
        "<sce:test-vector> at SCXML L46: into_owned field `payload` mismatch"
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
fn test_vector_codec_zenoh_frame_l50() {
    let actual = CodecZenohFrame {
        sn: 0x80u64,
        payload: b"\xde\xad",
    };
    let encoded = actual.encode_to_vec();
    let expected: &[u8] = &[0x80u8, 0x01u8, 0xdeu8, 0xadu8];
    assert_eq!(
        encoded.as_slice(), expected,
        "<sce:test-vector> at SCXML L50: encode produced {:?}, expected {:?}",
        encoded, expected
    );
    let mut cursor = SceCursor::new(expected);
    let decoded = CodecZenohFrame::decode(&mut cursor)
        .expect("<sce:test-vector> at SCXML L50: decode failed");
    assert_eq!(
        cursor.remaining(), 0,
        "<sce:test-vector> at SCXML L50: decode left {} bytes unconsumed",
        cursor.remaining()
    );
    assert_eq!(
        decoded.sn, 0x80u64,
        "<sce:test-vector> at SCXML L50: field `sn` mismatch"
    );
    assert_eq!(
        decoded.payload, b"\xde\xad",
        "<sce:test-vector> at SCXML L50: field `payload` mismatch"
    );
    // Owned-projection round-trip (consumer-requested; acceptance #2):
    // deep-copy the borrowed decode into its owned mirror and assert
    // every field still equals the oracle. Owned `Vec<u8>` / `String`
    // fields compare directly against the `&[u8]` / `&str` literals.
    let owned = CodecZenohFrame::decode(&mut SceCursor::new(expected))
        .expect("<sce:test-vector> at SCXML L50: decode for into_owned failed")
        .into_owned();
    assert_eq!(
        owned.sn, 0x80u64,
        "<sce:test-vector> at SCXML L50: into_owned field `sn` mismatch"
    );
    assert_eq!(
        owned.payload, b"\xde\xad",
        "<sce:test-vector> at SCXML L50: into_owned field `payload` mismatch"
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
