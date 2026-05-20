// SCE Forge: Auto-generated codec test-vector sidecar (RFC §5.B B5-θ)
// Companion to codec_zenoh_frame.rs — do not edit; regenerate from the source SCXML.

#[test]
fn test_vector_codec_zenoh_frame_l38() {
    let actual = CodecZenohFrame {
        sn: 0x0u64,
        payload: Vec::<u8>::new(),
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
        decoded.payload, Vec::<u8>::new(),
        "<sce:test-vector> at SCXML L38: field `payload` mismatch"
    );
}
#[test]
fn test_vector_codec_zenoh_frame_l42() {
    let actual = CodecZenohFrame {
        sn: 0x1u64,
        payload: vec![0xca, 0xfe],
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
        decoded.payload, vec![0xca, 0xfe],
        "<sce:test-vector> at SCXML L42: field `payload` mismatch"
    );
}
#[test]
fn test_vector_codec_zenoh_frame_l46() {
    let actual = CodecZenohFrame {
        sn: 0x7fu64,
        payload: vec![0xaa, 0xbb, 0xcc],
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
        decoded.payload, vec![0xaa, 0xbb, 0xcc],
        "<sce:test-vector> at SCXML L46: field `payload` mismatch"
    );
}
#[test]
fn test_vector_codec_zenoh_frame_l50() {
    let actual = CodecZenohFrame {
        sn: 0x80u64,
        payload: vec![0xde, 0xad],
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
        decoded.payload, vec![0xde, 0xad],
        "<sce:test-vector> at SCXML L50: field `payload` mismatch"
    );
}
