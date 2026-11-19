// SCE-GENERATED — DO NOT EDIT
// source-hash: 14e606f3cf1e7812017b5644cfc8eae8583c21d3a88314c7ab2afa22685d6826
// source: sce:std/sync/sync_delete_outcome.scxml
// SCE-MAP: sync_delete_outcome.scxml:28 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `SyncDeleteOutcome`, a file of the
// module that calls it: a call is qualified by the enum
// (`SyncDeleteOutcome.syncDeleteOutcome(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum SyncDeleteOutcome {
    public static func syncDeleteOutcome(_ kind: UInt8, _ status: Int32) throws(AlgorithmError) -> UInt8 {
        if !(kind >= 1 && kind <= 9) {
            throw AlgorithmError.precondition
        }
        if !(kind == 4 ? status >= 200 && status <= 599 : status == 0) {
            throw AlgorithmError.precondition
        }
        var outcome: UInt8 = 2
        if kind == 4 && status < 500 {
            outcome = status < 300 || status == 404 || status == 410 ? 0 : 1
        }
        return outcome
    }
}
