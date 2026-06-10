// SCE Forge: Auto-generated test-vector sidecar (RFC §synth-5-B B2)
// Companion to algorithm_crc16.rs — do not edit; regenerate from the source SCXML.

#[test]
fn test_vector_algorithm_crc16_l47() {
    let actual = algorithm_crc16(&[0x31u8, 0x32u8, 0x33u8, 0x34u8, 0x35u8, 0x36u8, 0x37u8, 0x38u8, 0x39u8]);
    let expected: u16 = 0x29b1u16;
    assert_eq!(
        actual, expected,
        "<sce:test-vector> at SCXML L47: algorithm_crc16(<313233343536373839>) returned {actual:?}, expected {expected:?}"
    );
}
