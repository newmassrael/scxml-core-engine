// SCE-GENERATED — DO NOT EDIT
// source-hash: 7cb5c601bc5bd6648425cb4d614d7be9f82d1e41816b0f69f2fcd5bc5c43d40b
// SCE-MAP: algorithm_list_param_sum.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `AlgorithmListParamSum`, a file of the
// module that calls it: a call is qualified by the enum
// (`AlgorithmListParamSum.algorithmListParamSum(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum AlgorithmListParamSum {
    public static func algorithmListParamSum(_ xs: [Int32]) throws(AlgorithmError) -> Int64 {
        var s: Int64 = 0
        for x in xs {
            s = (try Checked.add(s, Int64(x)))
        }
        let n: UInt32 = UInt32(truncatingIfNeeded: xs.count)
        if n > 0 {
            s = (try Checked.add(s, Int64((try Checked.at(xs, (try Checked.sub(n, 1)))))))
        }
        return s
    }
}
