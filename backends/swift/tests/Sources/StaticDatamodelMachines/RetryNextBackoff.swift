// SCE-GENERATED — DO NOT EDIT
// source-hash: 3d8e77ae6e1148d1818998f2f9d129b014c767dc3cded9cb0af2d4fc0c10af0a
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
    public static func retryNextBackoff(_ prevMs: Int64, _ multiplier: Double, _ maxMs: Int64) throws(AlgorithmError) -> Int64 {
        if !(multiplier >= 1.0 && prevMs > 0 && maxMs >= prevMs) {
            throw AlgorithmError.precondition
        }
        let grown: Double = Double(prevMs) * multiplier
        var next: Int64 = maxMs
        if grown < Double(maxMs) {
            next = Real.floor(grown, to: Int64.self)
        }
        return next
    }
}
