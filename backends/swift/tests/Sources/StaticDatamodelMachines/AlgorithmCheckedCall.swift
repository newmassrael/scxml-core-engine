// SCE-GENERATED — DO NOT EDIT
// source-hash: 1ba579cdc10cbe049a3766817dcc6397d58e8716381e63be0e21ee74788ac719
// SCE-MAP: algorithm_checked_call.scxml:16 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `AlgorithmCheckedCall`, a file of the
// module that calls it: a call is qualified by the enum
// (`AlgorithmCheckedCall.algorithmCheckedCall(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum AlgorithmCheckedCall {
    public static func algorithmCheckedCall(_ a: Int32, _ b: Int32, _ op: UInt8) throws -> Int32 {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        Checked.take(sceFailure, fallback: 0, { try AlgorithmCheckedArith.algorithmCheckedArith(a, b, op) })
        try sceFailure.throwIfFailed()
        let s: Int32 = Checked.take(sceFailure, fallback: 0, { try AlgorithmCheckedArith.algorithmCheckedArith(a, b, 0) })
        try sceFailure.throwIfFailed()
        do {
            let sceCond2 = Checked.take(sceFailure, fallback: 0, { try AlgorithmCheckedArith.algorithmCheckedArith(s, s, 2) }) > 100
            try sceFailure.throwIfFailed()
            if sceCond2 {
                do {
                    let sceReturned: Int32 = 0
                    try sceFailure.throwIfFailed()
                    return sceReturned
                }
            }
        }
        do {
            let sceReturned: Int32 = Checked.add(sceFailure, s, 1)
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
