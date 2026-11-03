// SCE-GENERATED — DO NOT EDIT
// source-hash: c868fafd129dfb9c6768c3d112e14542a926c118aae9597ed39656faf85e6076
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
    public static func retryExhausted(_ attempts: UInt32, _ maxRetries: UInt32, _ retryable: Bool) throws -> Bool {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        do {
            let sceCond2 = !(maxRetries > 0)
            try sceFailure.throwIfFailed()
            if sceCond2 {
                throw AlgorithmError.precondition
            }
        }
        do {
            let sceReturned: Bool = !retryable || attempts > maxRetries
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
