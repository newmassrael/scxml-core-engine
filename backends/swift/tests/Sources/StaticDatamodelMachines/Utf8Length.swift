// SCE-GENERATED — DO NOT EDIT
// source-hash: 848898b5bbb016bf799fdf5a1235f8e512e7516e53f20e55bfd5887716ace728
// SCE-MAP: utf8_length.scxml:56 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `Utf8Length`, a file of the
// module that calls it: a call is qualified by the enum
// (`Utf8Length.utf8Length(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum Utf8Length {
    public static func utf8Length(_ data: [UInt8]) throws(AlgorithmError) -> UInt32 {
        let n: UInt32 = UInt32(truncatingIfNeeded: data.count)
        var i: UInt32 = 0
        var count: UInt32 = 0
        while i < n {
            let lead: UInt32 = UInt32((try Checked.at(data, i)))
            var tails: UInt32 = 0
            var lo: UInt32 = 128
            var hi: UInt32 = 191
            if lead <= 127 {
                tails = 0
            } else {
                if lead >= 194 && lead <= 223 {
                    tails = 1
                } else {
                    if lead == 224 {
                        tails = 2
                        lo = 160
                    } else {
                        if lead == 237 {
                            tails = 2
                            hi = 159
                        } else {
                            if lead >= 225 && lead <= 239 {
                                tails = 2
                            } else {
                                if lead == 240 {
                                    tails = 3
                                    lo = 144
                                } else {
                                    if lead == 244 {
                                        tails = 3
                                        hi = 143
                                    } else {
                                        if !(lead >= 241 && lead <= 243) {
                                            throw AlgorithmError.precondition
                                        }
                                        tails = 3
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if !(tails <= (try Checked.sub((try Checked.sub(n, i)), 1))) {
                throw AlgorithmError.precondition
            }
            var k: UInt32 = 1
            while k <= tails {
                let b: UInt32 = UInt32((try Checked.at(data, (try Checked.add(i, k)))))
                if !(b >= (k == 1 ? lo : 128) && b <= (k == 1 ? hi : 191)) {
                    throw AlgorithmError.precondition
                }
                k = (try Checked.add(k, 1))
            }
            i = (try Checked.add((try Checked.add(i, tails)), 1))
            count = (try Checked.add(count, 1))
        }
        return count
    }
}
