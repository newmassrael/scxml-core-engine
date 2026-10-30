// SCE-GENERATED — DO NOT EDIT
// source-hash: f6c860c2693fcb3366bc871ff801c9b0ee9e491a1759830f8d83238267cc3b45
// SCE-MAP: algorithm_total_arith_i32.scxml:16 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `AlgorithmTotalArithI32`, a file of the
// module that calls it: a call is qualified by the enum
// (`AlgorithmTotalArithI32.algorithmTotalArithI32(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum AlgorithmTotalArithI32 {
    public static func algorithmTotalArithI32(_ a: Int32, _ b: Int32, _ op: UInt8) -> Int32 {
        var r: Int32 = 0
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
        if op == 7 {
            r = Saturating.neg(a)
        }
        return r
    }
}
