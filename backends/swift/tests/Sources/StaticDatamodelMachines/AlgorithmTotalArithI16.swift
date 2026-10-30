// SCE-GENERATED — DO NOT EDIT
// source-hash: 1ba579cdc10cbe049a3766817dcc6397d58e8716381e63be0e21ee74788ac719
// SCE-MAP: algorithm_total_arith_i16.scxml:17 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `AlgorithmTotalArithI16`, a file of the
// module that calls it: a call is qualified by the enum
// (`AlgorithmTotalArithI16.algorithmTotalArithI16(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum AlgorithmTotalArithI16 {
    public static func algorithmTotalArithI16(_ a: Int32, _ b: Int32, _ op: UInt8) -> Int32 {
        var r: Int16 = 0
        if a >= -32768 && a <= 32767 && b >= -32768 && b <= 32767 {
            let x: Int16 = Int16(truncatingIfNeeded: a)
            let y: Int16 = Int16(truncatingIfNeeded: b)
            if op == 0 {
                r = Wrapping.add(x, y)
            }
            if op == 1 {
                r = Wrapping.sub(x, y)
            }
            if op == 2 {
                r = Wrapping.mul(x, y)
            }
            if op == 3 {
                r = Wrapping.neg(x)
            }
            if op == 4 {
                r = Saturating.add(x, y)
            }
            if op == 5 {
                r = Saturating.sub(x, y)
            }
            if op == 6 {
                r = Saturating.mul(x, y)
            }
            if op == 7 {
                r = Saturating.neg(x)
            }
        }
        return Int32(r)
    }
}
