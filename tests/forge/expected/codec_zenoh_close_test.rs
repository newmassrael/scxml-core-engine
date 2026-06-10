// SCE Forge: Auto-generated codec test-vector sidecar (RFC §5.B)
// Companion to codec_zenoh_close.rs — do not edit; regenerate from the source SCXML.

#[test]
fn test_vector_codec_zenoh_close_l27() {
    let actual = CodecZenohClose {
        reason: 0x0u8,
    };
    let encoded = actual.encode_to_vec();
    let expected: &[u8] = &[0x00u8];
    assert_eq!(
        encoded.as_slice(), expected,
        "<sce:test-vector> at SCXML L27: encode produced {:?}, expected {:?}",
        encoded, expected
    );
    let mut cursor = SceCursor::new(expected);
    let decoded = CodecZenohClose::decode(&mut cursor)
        .expect("<sce:test-vector> at SCXML L27: decode failed");
    assert_eq!(
        cursor.remaining(), 0,
        "<sce:test-vector> at SCXML L27: decode left {} bytes unconsumed",
        cursor.remaining()
    );
    assert_eq!(
        decoded.reason, 0x0u8,
        "<sce:test-vector> at SCXML L27: field `reason` mismatch"
    );
}
#[test]
fn test_vector_codec_zenoh_close_l30() {
    let actual = CodecZenohClose {
        reason: 0x1u8,
    };
    let encoded = actual.encode_to_vec();
    let expected: &[u8] = &[0x01u8];
    assert_eq!(
        encoded.as_slice(), expected,
        "<sce:test-vector> at SCXML L30: encode produced {:?}, expected {:?}",
        encoded, expected
    );
    let mut cursor = SceCursor::new(expected);
    let decoded = CodecZenohClose::decode(&mut cursor)
        .expect("<sce:test-vector> at SCXML L30: decode failed");
    assert_eq!(
        cursor.remaining(), 0,
        "<sce:test-vector> at SCXML L30: decode left {} bytes unconsumed",
        cursor.remaining()
    );
    assert_eq!(
        decoded.reason, 0x1u8,
        "<sce:test-vector> at SCXML L30: field `reason` mismatch"
    );
}
#[test]
fn test_vector_codec_zenoh_close_l33() {
    let actual = CodecZenohClose {
        reason: 0x2u8,
    };
    let encoded = actual.encode_to_vec();
    let expected: &[u8] = &[0x02u8];
    assert_eq!(
        encoded.as_slice(), expected,
        "<sce:test-vector> at SCXML L33: encode produced {:?}, expected {:?}",
        encoded, expected
    );
    let mut cursor = SceCursor::new(expected);
    let decoded = CodecZenohClose::decode(&mut cursor)
        .expect("<sce:test-vector> at SCXML L33: decode failed");
    assert_eq!(
        cursor.remaining(), 0,
        "<sce:test-vector> at SCXML L33: decode left {} bytes unconsumed",
        cursor.remaining()
    );
    assert_eq!(
        decoded.reason, 0x2u8,
        "<sce:test-vector> at SCXML L33: field `reason` mismatch"
    );
}
#[test]
fn test_vector_codec_zenoh_close_l36() {
    let actual = CodecZenohClose {
        reason: 0xffu8,
    };
    let encoded = actual.encode_to_vec();
    let expected: &[u8] = &[0xffu8];
    assert_eq!(
        encoded.as_slice(), expected,
        "<sce:test-vector> at SCXML L36: encode produced {:?}, expected {:?}",
        encoded, expected
    );
    let mut cursor = SceCursor::new(expected);
    let decoded = CodecZenohClose::decode(&mut cursor)
        .expect("<sce:test-vector> at SCXML L36: decode failed");
    assert_eq!(
        cursor.remaining(), 0,
        "<sce:test-vector> at SCXML L36: decode left {} bytes unconsumed",
        cursor.remaining()
    );
    assert_eq!(
        decoded.reason, 0xffu8,
        "<sce:test-vector> at SCXML L36: field `reason` mismatch"
    );
}
