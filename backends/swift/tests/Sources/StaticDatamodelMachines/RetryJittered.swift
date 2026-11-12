// SCE-GENERATED — DO NOT EDIT
// source-hash: 3d8e77ae6e1148d1818998f2f9d129b014c767dc3cded9cb0af2d4fc0c10af0a
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
    public static func retryJittered(_ baseMs: Int64, _ jitterPct: Int64, _ draw: Int64) throws(AlgorithmError) -> Int64 {
        if !(baseMs > 0 && jitterPct >= 0 && jitterPct <= 100 && draw >= 0) {
            throw AlgorithmError.precondition
        }
        let delta: Int64 = (try Checked.add((try Checked.mul((try Checked.div(baseMs, 100)), jitterPct)), (try Checked.div((try Checked.mul((try Checked.rem(baseMs, 100)), jitterPct)), 100))))
        let waited: Int64 = (try Checked.add((try Checked.sub(baseMs, delta)), (try Checked.rem(draw, (try Checked.add((try Checked.mul(2, delta)), 1))))))
        return waited < 1 ? 1 : waited
    }
}
