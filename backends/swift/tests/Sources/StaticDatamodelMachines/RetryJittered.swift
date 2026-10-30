// SCE-GENERATED — DO NOT EDIT
// source-hash: 3e850e348999a821018727a6eab60f939f8f51a6e8133ca476ace471d64c40a6
// SCE-MAP: retry_jittered.scxml:23 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `RetryJittered`, a file of the
// module that calls it: a call is qualified by the enum
// (`RetryJittered.retryJittered(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum RetryJittered {
    public static func retryJittered(_ baseMs: Int64, _ jitterPct: Int64, _ draw: Int64) throws -> Int64 {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        do {
            let sceCond2 = !(baseMs > 0 && jitterPct >= 0 && jitterPct <= 100 && draw >= 0)
            try sceFailure.throwIfFailed()
            if sceCond2 {
                throw AlgorithmError.precondition
            }
        }
        let delta: Int64 = Checked.add(sceFailure, Checked.mul(sceFailure, Checked.div(sceFailure, baseMs, 100), jitterPct), Checked.div(sceFailure, Checked.mul(sceFailure, Checked.rem(sceFailure, baseMs, 100), jitterPct), 100))
        try sceFailure.throwIfFailed()
        let waited: Int64 = Checked.add(sceFailure, Checked.sub(sceFailure, baseMs, delta), Checked.rem(sceFailure, draw, Checked.add(sceFailure, Checked.mul(sceFailure, 2, delta), 1)))
        try sceFailure.throwIfFailed()
        do {
            let sceReturned: Int64 = waited < 1 ? 1 : waited
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
