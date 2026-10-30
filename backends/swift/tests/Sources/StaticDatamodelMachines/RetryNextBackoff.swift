// SCE-GENERATED — DO NOT EDIT
// source-hash: 3e850e348999a821018727a6eab60f939f8f51a6e8133ca476ace471d64c40a6
// SCE-MAP: retry_next_backoff.scxml:19 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `RetryNextBackoff`, a file of the
// module that calls it: a call is qualified by the enum
// (`RetryNextBackoff.retryNextBackoff(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum RetryNextBackoff {
    public static func retryNextBackoff(_ prevMs: Int64, _ multiplier: Double, _ maxMs: Int64) throws -> Int64 {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        do {
            let sceCond2 = !(multiplier >= 1.0 && prevMs > 0 && maxMs >= prevMs)
            try sceFailure.throwIfFailed()
            if sceCond2 {
                throw AlgorithmError.precondition
            }
        }
        let grown: Double = Double(prevMs) * multiplier
        try sceFailure.throwIfFailed()
        var next: Int64 = maxMs
        try sceFailure.throwIfFailed()
        do {
            let sceCond2 = grown < Double(maxMs)
            try sceFailure.throwIfFailed()
            if sceCond2 {
                next = Real.floor(grown, to: Int64.self)
                try sceFailure.throwIfFailed()
            }
        }
        do {
            let sceReturned: Int64 = next
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
