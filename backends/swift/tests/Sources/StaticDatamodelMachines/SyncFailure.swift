// SCE-GENERATED — DO NOT EDIT
// source-hash: 9e46d5ba618e2f5a559e7793b0e0ddd1d7ba8428fd3d96128f06266cf4750b56
// SCE-MAP: sync_failure.scxml:30 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `SyncFailure`, a file of the
// module that calls it: a call is qualified by the enum
// (`SyncFailure.syncFailure(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum SyncFailure {
    public static func syncFailure(_ kind: UInt8, _ status: Int32) throws(AlgorithmError) -> UInt8 {
        if !(kind >= 1 && kind <= 9) {
            throw AlgorithmError.precondition
        }
        if !(kind == 4 ? status >= 300 && status <= 599 : status == 0) {
            throw AlgorithmError.precondition
        }
        var action: UInt8 = 2
        if kind == 8 || kind == 1 || kind == 2 {
            action = 1
        }
        if kind == 7 {
            action = 4
        }
        if kind == 3 {
            action = 0
        }
        if kind == 4 && status == 401 {
            action = 3
        }
        if kind == 4 && (status == 503 || status == 502) {
            action = 1
        }
        return action
    }
}
