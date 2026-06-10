// SCE Forge: Auto-generated test-vector sidecar (RFC §synth-5-B B2)
// Companion to algorithm_crc16.go — do not edit; regenerate from the source SCXML.

package algorithm_crc16

import "testing"

func TestVectorAlgorithmCrc16L47(t *testing.T) {
    actual := AlgorithmCrc16([]byte{0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39})
    expected := uint16(0x29b1)
    if actual != expected {
        t.Errorf(
            "<sce:test-vector> at SCXML L47: AlgorithmCrc16(<313233343536373839>) = %v, want %v",
            actual, expected)
    }
}
