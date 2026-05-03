// SCE Forge: Auto-generated codec test-vector sidecar (RFC §5.B B5-θ)
// Companion to codec_length_ref_dotted_basic.rs — do not edit; regenerate from the source SCXML.

#[test]
fn test_vector_codec_length_ref_dotted_basic_l41() {
    let actual = CodecLengthRefDottedBasic {
        carrier: 0x0u8,
        payload: Vec::<u8>::new(),
    };
    let encoded = actual.encode();
    let expected: &[u8] = &[0x00u8];
    assert_eq!(
        encoded.as_slice(), expected,
        "<sce:test-vector> at SCXML L41: encode produced {:?}, expected {:?}",
        encoded, expected
    );
    let mut cursor = SceCursor::new(expected);
    let decoded = CodecLengthRefDottedBasic::decode(&mut cursor)
        .expect("<sce:test-vector> at SCXML L41: decode failed");
    assert_eq!(
        cursor.remaining(), 0,
        "<sce:test-vector> at SCXML L41: decode left {} bytes unconsumed",
        cursor.remaining()
    );
    assert_eq!(
        decoded.carrier, 0x0u8,
        "<sce:test-vector> at SCXML L41: field `carrier` mismatch"
    );
    assert_eq!(
        decoded.payload, Vec::<u8>::new(),
        "<sce:test-vector> at SCXML L41: field `payload` mismatch"
    );
}
#[test]
fn test_vector_codec_length_ref_dotted_basic_l45() {
    let actual = CodecLengthRefDottedBasic {
        carrier: 0x21u8,
        payload: vec![0xaa, 0xbb],
    };
    let encoded = actual.encode();
    let expected: &[u8] = &[0x21u8, 0xaau8, 0xbbu8];
    assert_eq!(
        encoded.as_slice(), expected,
        "<sce:test-vector> at SCXML L45: encode produced {:?}, expected {:?}",
        encoded, expected
    );
    let mut cursor = SceCursor::new(expected);
    let decoded = CodecLengthRefDottedBasic::decode(&mut cursor)
        .expect("<sce:test-vector> at SCXML L45: decode failed");
    assert_eq!(
        cursor.remaining(), 0,
        "<sce:test-vector> at SCXML L45: decode left {} bytes unconsumed",
        cursor.remaining()
    );
    assert_eq!(
        decoded.carrier, 0x21u8,
        "<sce:test-vector> at SCXML L45: field `carrier` mismatch"
    );
    assert_eq!(
        decoded.payload, vec![0xaa, 0xbb],
        "<sce:test-vector> at SCXML L45: field `payload` mismatch"
    );
}
#[test]
fn test_vector_codec_length_ref_dotted_basic_l49() {
    let actual = CodecLengthRefDottedBasic {
        carrier: 0xf5u8,
        payload: vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e],
    };
    let encoded = actual.encode();
    let expected: &[u8] = &[0xf5u8, 0x00u8, 0x01u8, 0x02u8, 0x03u8, 0x04u8, 0x05u8, 0x06u8, 0x07u8, 0x08u8, 0x09u8, 0x0au8, 0x0bu8, 0x0cu8, 0x0du8, 0x0eu8];
    assert_eq!(
        encoded.as_slice(), expected,
        "<sce:test-vector> at SCXML L49: encode produced {:?}, expected {:?}",
        encoded, expected
    );
    let mut cursor = SceCursor::new(expected);
    let decoded = CodecLengthRefDottedBasic::decode(&mut cursor)
        .expect("<sce:test-vector> at SCXML L49: decode failed");
    assert_eq!(
        cursor.remaining(), 0,
        "<sce:test-vector> at SCXML L49: decode left {} bytes unconsumed",
        cursor.remaining()
    );
    assert_eq!(
        decoded.carrier, 0xf5u8,
        "<sce:test-vector> at SCXML L49: field `carrier` mismatch"
    );
    assert_eq!(
        decoded.payload, vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e],
        "<sce:test-vector> at SCXML L49: field `payload` mismatch"
    );
}
