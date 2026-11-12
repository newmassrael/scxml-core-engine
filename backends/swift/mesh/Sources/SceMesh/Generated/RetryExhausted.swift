// SCE-GENERATED — DO NOT EDIT
// source-hash: f2f8682669670c7aaedbccb3102dd71d7b187eac29a32d3d20509a5bc070c12e
// SCE-MAP: retry_exhausted.scxml:17 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `RetryExhausted`, a file of the
// module that calls it: a call is qualified by the enum
// (`RetryExhausted.retryExhausted(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum RetryExhausted {
    public static func retryExhausted(_ attempts: UInt32, _ maxRetries: UInt32, _ retryable: Bool) throws(AlgorithmError) -> Bool {
        if !(maxRetries > 0) {
            throw AlgorithmError.precondition
        }
        return !retryable || attempts > maxRetries
    }
}
