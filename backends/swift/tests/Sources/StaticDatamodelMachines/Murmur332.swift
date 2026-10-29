// SCE-GENERATED — DO NOT EDIT
// source-hash: 39030e6106e848245cd6e12123770f8e044c8cc2e2eac9ec2102ed1dd561bf26
// SCE-MAP: murmur3_32.scxml:27 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `Murmur332`, a file of the
// module that calls it: a call is qualified by the enum
// (`Murmur332.murmur332(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum Murmur332 {
    public static func murmur332(_ data: [UInt8], _ seed: UInt32) -> UInt32 {
        let n: UInt32 = UInt32(truncatingIfNeeded: data.count)
        var h: UInt64 = UInt64(seed)
        var k: UInt64 = 0
        var j: UInt32 = 0
        while j < n {
            let b: UInt64 = UInt64(data[Int(j)])
            k = (k | (b << (8 * (j & 3)))) & 0xFFFFFFFF
            if (j & 3) == 3 {
                k = ((k & 0xFFFFFFFF) * 0xcc9e2d51) & 0xFFFFFFFF
                k = ((k << 15) | (k >> 17)) & 0xFFFFFFFF
                k = ((k & 0xFFFFFFFF) * 0x1b873593) & 0xFFFFFFFF
                h = (h ^ k) & 0xFFFFFFFF
                h = ((h << 13) | (h >> 19)) & 0xFFFFFFFF
                h = ((h & 0xFFFFFFFF) * 5 + 0xe6546b64) & 0xFFFFFFFF
                k = 0
            }
            j = j + 1
        }
        if (n & 3) != 0 {
            k = ((k & 0xFFFFFFFF) * 0xcc9e2d51) & 0xFFFFFFFF
            k = ((k << 15) | (k >> 17)) & 0xFFFFFFFF
            k = ((k & 0xFFFFFFFF) * 0x1b873593) & 0xFFFFFFFF
            h = (h ^ k) & 0xFFFFFFFF
        }
        h = (h ^ UInt64(n)) & 0xFFFFFFFF
        h = (h ^ (h >> 16)) & 0xFFFFFFFF
        h = ((h & 0xFFFFFFFF) * 0x85ebca6b) & 0xFFFFFFFF
        h = (h ^ (h >> 13)) & 0xFFFFFFFF
        h = ((h & 0xFFFFFFFF) * 0xc2b2ae35) & 0xFFFFFFFF
        h = (h ^ (h >> 16)) & 0xFFFFFFFF
        return UInt32(truncatingIfNeeded: h & 0xFFFFFFFF)
    }
}
