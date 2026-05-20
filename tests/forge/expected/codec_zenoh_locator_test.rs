// SCE Forge: Auto-generated codec test-vector sidecar (RFC §5.B B5-θ)
// Companion to codec_zenoh_locator.rs — do not edit; regenerate from the source SCXML.

#[test]
fn test_vector_codec_zenoh_locator_l38() {
    let actual = CodecZenohLocator {
        locator_len: 0x0u64,
        locator: String::from(""),
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
        decoded.locator, String::from(""),
        "<sce:test-vector> at SCXML L38: field `locator` mismatch"
    );
}
#[test]
fn test_vector_codec_zenoh_locator_l42() {
    let actual = CodecZenohLocator {
        locator_len: 0x3u64,
        locator: String::from("abc"),
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
        decoded.locator, String::from("abc"),
        "<sce:test-vector> at SCXML L42: field `locator` mismatch"
    );
}
#[test]
fn test_vector_codec_zenoh_locator_l46() {
    let actual = CodecZenohLocator {
        locator_len: 0x12u64,
        locator: String::from("tcp/127.0.0.1:7447"),
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
        decoded.locator, String::from("tcp/127.0.0.1:7447"),
        "<sce:test-vector> at SCXML L46: field `locator` mismatch"
    );
}
#[test]
fn test_vector_codec_zenoh_locator_l50() {
    let actual = CodecZenohLocator {
        locator_len: 0x6u64,
        locator: String::from("héllo"),
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
        decoded.locator, String::from("héllo"),
        "<sce:test-vector> at SCXML L50: field `locator` mismatch"
    );
}
