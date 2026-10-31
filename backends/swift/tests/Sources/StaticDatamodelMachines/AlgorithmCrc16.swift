// SCE-GENERATED — DO NOT EDIT
// source-hash: f6c860c2693fcb3366bc871ff801c9b0ee9e491a1759830f8d83238267cc3b45
// SCE-MAP: algorithm_crc16.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `AlgorithmCrc16`, a file of the
// module that calls it: a call is qualified by the enum
// (`AlgorithmCrc16.algorithmCrc16(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum AlgorithmCrc16 {
    public static func algorithmCrc16(_ data: [UInt8]) -> UInt16 {
        var crc: UInt16 = 0xFFFF
        for b in data {
            let hi: UInt16 = UInt16(b)
            crc = crc ^ (hi << 8)
            var i: UInt8 = 0
            while i < 8 {
                if (crc & 0x8000) != 0 {
                    crc = (crc << 1) ^ 0x1021
                } else {
                    crc = crc << 1
                }
                i = i + 1
            }
        }
        return crc
    }
}
