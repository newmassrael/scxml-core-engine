// SCE-GENERATED — DO NOT EDIT
// source-hash: 1ba579cdc10cbe049a3766817dcc6397d58e8716381e63be0e21ee74788ac719
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
    public static func algorithmListParamSum(_ xs: [Int32]) throws -> Int64 {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        var s: Int64 = 0
        try sceFailure.throwIfFailed()
        for x in xs {
            s = Checked.add(sceFailure, s, Int64(x))
            try sceFailure.throwIfFailed()
        }
        let n: UInt32 = UInt32(truncatingIfNeeded: xs.count)
        try sceFailure.throwIfFailed()
        do {
            let sceCond2 = n > 0
            try sceFailure.throwIfFailed()
            if sceCond2 {
                s = Checked.add(sceFailure, s, Int64(Checked.at(sceFailure, xs, Checked.sub(sceFailure, n, 1), fallback: 0)))
                try sceFailure.throwIfFailed()
            }
        }
        do {
            let sceReturned: Int64 = s
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
