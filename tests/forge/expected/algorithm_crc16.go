// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §5.A: pure synchronous function with bounded loops. Free
// function in package `algorithm_crc16`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §5.J.5 emitter table).

package algorithm_crc16

func AlgorithmCrc16(data []byte) uint16 {
    var crc uint16 = 0xFFFF
    for _, b := range data {
        var hi uint16 = uint16(b)
        crc = crc ^ hi << 8;
        var i uint8 = 0
        for i < 8 {
            if (crc & 0x8000 != 0) {
                crc = crc << 1 ^ 0x1021;
            } else {
                crc = crc << 1;
            }
            i = i + 1;
        }
    }
    return crc;
}
