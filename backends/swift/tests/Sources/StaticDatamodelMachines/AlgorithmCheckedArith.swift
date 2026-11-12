// SCE-GENERATED — DO NOT EDIT
// source-hash: f3a316aacf7403325399aec899293549220a4c98b78ad03825aff15e55acb6a9
// SCE-MAP: algorithm_checked_arith.scxml:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `AlgorithmCheckedArith`, a file of the
// module that calls it: a call is qualified by the enum
// (`AlgorithmCheckedArith.algorithmCheckedArith(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum AlgorithmCheckedArith {
    public static func algorithmCheckedArith(_ a: Int32, _ b: Int32, _ op: UInt8) throws(AlgorithmError) -> Int32 {
        var r: Int32 = 0
        if op == 0 {
            r = (try Checked.add(a, b))
        }
        if op == 1 {
            r = (try Checked.sub(a, b))
        }
        if op == 2 {
            r = (try Checked.mul(a, b))
        }
        if op == 3 {
            r = (try Checked.div(a, b))
        }
        if op == 4 {
            r = (try Checked.rem(a, b))
        }
        if op == 5 {
            r = (try Checked.neg(a))
        }
        if op == 6 {
            r = Int32((try Checked.sub(op, 7)))
        }
        return r
    }
}
