// SCE-GENERATED — DO NOT EDIT
// source-hash: 10b81d4a2624d4eebdd1a7de3f50330977b7844a01609684dd7f080dfebf5faa
// source: algorithm_literal_arith.scxml
// SCE-MAP: algorithm_literal_arith.scxml:22 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `AlgorithmLiteralArith`, a file of the
// module that calls it: a call is qualified by the enum
// (`AlgorithmLiteralArith.algorithmLiteralArith(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum AlgorithmLiteralArith {
    public static func algorithmLiteralArith(_ prev: UInt64, _ m: UInt8, _ op: UInt8) throws(AlgorithmError) -> UInt64 {
        var r: UInt64 = prev
        if op == 0 {
            r = (try Checked.add(prev, (try Checked.mul(UInt64(65536), UInt64(65536)))))
        }
        if op == 1 {
            r = prev & (try Checked.sub((UInt64(1) << m), UInt64(1)))
        }
        if op == 2 {
            r = (try Checked.sub(prev, (UInt64(1) << m)))
        }
        if op == 3 {
            r = (try Checked.mul((UInt64(1) << m), UInt64(3)))
        }
        if op == 4 {
            r = (try Checked.add((UInt64(1) << m) >> 1, UInt64(1)))
        }
        return r
    }
}
