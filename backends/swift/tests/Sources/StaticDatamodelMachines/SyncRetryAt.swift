// SCE-GENERATED — DO NOT EDIT
// source-hash: 9e46d5ba618e2f5a559e7793b0e0ddd1d7ba8428fd3d96128f06266cf4750b56
// SCE-MAP: sync_retry_at.scxml:28 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `SyncRetryAt`, a file of the
// module that calls it: a call is qualified by the enum
// (`SyncRetryAt.syncRetryAt(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum SyncRetryAt {
    public static func syncRetryAt(_ previous: Int64, _ kind: UInt8, _ status: Int32, _ now: Int64, _ retryAfter: Int64) throws(AlgorithmError) -> Int64 {
        if !(kind >= 1 && kind <= 9) {
            throw AlgorithmError.precondition
        }
        if !(kind == 4 ? status >= 300 && status <= 599 : status == 0) {
            throw AlgorithmError.precondition
        }
        if !(previous >= 0 && now >= 0 && retryAfter >= 0) {
            throw AlgorithmError.precondition
        }
        var asked: Int64 = 0
        if kind == 4 && status == 503 && retryAfter > 0 {
            asked = (try Checked.add(now, retryAfter))
        }
        if kind == 4 && status == 502 {
            asked = (try Checked.add(now, 900))
        }
        return asked > previous ? asked : previous
    }
}
