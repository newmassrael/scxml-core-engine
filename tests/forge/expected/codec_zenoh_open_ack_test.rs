// SCE Forge: Auto-generated codec test-vector sidecar (RFC §5.B B5-θ)
// Companion to codec_zenoh_open_ack.rs — do not edit; regenerate from the source SCXML.

#[test]
fn test_vector_codec_zenoh_open_ack_l33() {
    let actual = CodecZenohOpenAck {
        lease: 0x0u64,
        initial_sn: 0x0u64,
    };
    let encoded = actual.encode();
    let expected: &[u8] = &[0x00u8, 0x00u8];
    assert_eq!(
        encoded.as_slice(), expected,
        "<sce:test-vector> at SCXML L33: encode produced {:?}, expected {:?}",
        encoded, expected
    );
    let mut cursor = SceCursor::new(expected);
    let decoded = CodecZenohOpenAck::decode(&mut cursor)
        .expect("<sce:test-vector> at SCXML L33: decode failed");
    assert_eq!(
        cursor.remaining(), 0,
        "<sce:test-vector> at SCXML L33: decode left {} bytes unconsumed",
        cursor.remaining()
    );
    assert_eq!(
        decoded.lease, 0x0u64,
        "<sce:test-vector> at SCXML L33: field `lease` mismatch"
    );
    assert_eq!(
        decoded.initial_sn, 0x0u64,
        "<sce:test-vector> at SCXML L33: field `initial_sn` mismatch"
    );
}
#[test]
fn test_vector_codec_zenoh_open_ack_l37() {
    let actual = CodecZenohOpenAck {
        lease: 0x1u64,
        initial_sn: 0x64u64,
    };
    let encoded = actual.encode();
    let expected: &[u8] = &[0x01u8, 0x64u8];
    assert_eq!(
        encoded.as_slice(), expected,
        "<sce:test-vector> at SCXML L37: encode produced {:?}, expected {:?}",
        encoded, expected
    );
    let mut cursor = SceCursor::new(expected);
    let decoded = CodecZenohOpenAck::decode(&mut cursor)
        .expect("<sce:test-vector> at SCXML L37: decode failed");
    assert_eq!(
        cursor.remaining(), 0,
        "<sce:test-vector> at SCXML L37: decode left {} bytes unconsumed",
        cursor.remaining()
    );
    assert_eq!(
        decoded.lease, 0x1u64,
        "<sce:test-vector> at SCXML L37: field `lease` mismatch"
    );
    assert_eq!(
        decoded.initial_sn, 0x64u64,
        "<sce:test-vector> at SCXML L37: field `initial_sn` mismatch"
    );
}
#[test]
fn test_vector_codec_zenoh_open_ack_l41() {
    let actual = CodecZenohOpenAck {
        lease: 0x7fu64,
        initial_sn: 0x1u64,
    };
    let encoded = actual.encode();
    let expected: &[u8] = &[0x7fu8, 0x01u8];
    assert_eq!(
        encoded.as_slice(), expected,
        "<sce:test-vector> at SCXML L41: encode produced {:?}, expected {:?}",
        encoded, expected
    );
    let mut cursor = SceCursor::new(expected);
    let decoded = CodecZenohOpenAck::decode(&mut cursor)
        .expect("<sce:test-vector> at SCXML L41: decode failed");
    assert_eq!(
        cursor.remaining(), 0,
        "<sce:test-vector> at SCXML L41: decode left {} bytes unconsumed",
        cursor.remaining()
    );
    assert_eq!(
        decoded.lease, 0x7fu64,
        "<sce:test-vector> at SCXML L41: field `lease` mismatch"
    );
    assert_eq!(
        decoded.initial_sn, 0x1u64,
        "<sce:test-vector> at SCXML L41: field `initial_sn` mismatch"
    );
}
#[test]
fn test_vector_codec_zenoh_open_ack_l45() {
    let actual = CodecZenohOpenAck {
        lease: 0x80u64,
        initial_sn: 0xc8u64,
    };
    let encoded = actual.encode();
    let expected: &[u8] = &[0x80u8, 0x01u8, 0xc8u8, 0x01u8];
    assert_eq!(
        encoded.as_slice(), expected,
        "<sce:test-vector> at SCXML L45: encode produced {:?}, expected {:?}",
        encoded, expected
    );
    let mut cursor = SceCursor::new(expected);
    let decoded = CodecZenohOpenAck::decode(&mut cursor)
        .expect("<sce:test-vector> at SCXML L45: decode failed");
    assert_eq!(
        cursor.remaining(), 0,
        "<sce:test-vector> at SCXML L45: decode left {} bytes unconsumed",
        cursor.remaining()
    );
    assert_eq!(
        decoded.lease, 0x80u64,
        "<sce:test-vector> at SCXML L45: field `lease` mismatch"
    );
    assert_eq!(
        decoded.initial_sn, 0xc8u64,
        "<sce:test-vector> at SCXML L45: field `initial_sn` mismatch"
    );
}
