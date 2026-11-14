// SCE-GENERATED — DO NOT EDIT
// source-hash: 1ee4c478fcd6af0bb675edd6c8e90348072aad4cd4e5fb9446a5e52706545b90
// SCE-MAP: algorithm_total_arith_u32.scxml:16 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `AlgorithmTotalArithU32`, a file of the
// module that calls it: a call is qualified by the enum
// (`AlgorithmTotalArithU32.algorithmTotalArithU32(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum AlgorithmTotalArithU32 {
    public static func algorithmTotalArithU32(_ a: UInt32, _ b: UInt32, _ op: UInt8) -> UInt32 {
        var r: UInt32 = 0
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
