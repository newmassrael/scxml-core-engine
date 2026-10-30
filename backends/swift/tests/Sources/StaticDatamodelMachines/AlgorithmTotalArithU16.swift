// SCE-GENERATED — DO NOT EDIT
// source-hash: 1ba579cdc10cbe049a3766817dcc6397d58e8716381e63be0e21ee74788ac719
// SCE-MAP: algorithm_total_arith_u16.scxml:16 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `AlgorithmTotalArithU16`, a file of the
// module that calls it: a call is qualified by the enum
// (`AlgorithmTotalArithU16.algorithmTotalArithU16(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum AlgorithmTotalArithU16 {
    public static func algorithmTotalArithU16(_ a: UInt16, _ b: UInt16, _ op: UInt8) -> UInt16 {
        var r: UInt16 = 0
        if op == 0 {
            r = Wrapping.add(a, b)
        }
        if op == 1 {
            r = Wrapping.sub(a, b)
        }
        if op == 2 {
            r = Wrapping.mul(a, b)
        }
        if op == 3 {
            r = Wrapping.neg(a)
        }
        if op == 4 {
            r = Saturating.add(a, b)
        }
        if op == 5 {
            r = Saturating.sub(a, b)
        }
        if op == 6 {
            r = Saturating.mul(a, b)
        }
        return r
    }
}
