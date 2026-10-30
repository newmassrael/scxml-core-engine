// SCE-GENERATED — DO NOT EDIT
// source-hash: f6c860c2693fcb3366bc871ff801c9b0ee9e491a1759830f8d83238267cc3b45
// SCE-MAP: algorithm_total_arith_i8.scxml:17 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `AlgorithmTotalArithI8`, a file of the
// module that calls it: a call is qualified by the enum
// (`AlgorithmTotalArithI8.algorithmTotalArithI8(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum AlgorithmTotalArithI8 {
    public static func algorithmTotalArithI8(_ a: Int32, _ b: Int32, _ op: UInt8) -> Int32 {
        var r: Int8 = 0
        if a >= -128 && a <= 127 && b >= -128 && b <= 127 {
            let x: Int8 = Int8(truncatingIfNeeded: a)
            let y: Int8 = Int8(truncatingIfNeeded: b)
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
