// SCE-GENERATED — DO NOT EDIT
// source-hash: 8fa41ba4d02ec0c660aae577c6604c3b5470da58c62da65404cc05ef8dc67bfe
// SCE-MAP: sync_delete_outcome.scxml:28 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `sync_delete_outcome`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package sync_delete_outcome

import (
	scealgorithm "github.com/newmassrael/sce-forge-runtime/algorithm"
)

func SyncDeleteOutcome(kind uint8, status int32) (sceValue uint8, sceErr error) {
    // SCE_FORGE.md §3.4.1: each checked operation records a failure here, and
    // the statement around it returns it before the next statement runs.
    var sceFailure scealgorithm.Failure
    if sceCond1 := !(kind >= 1 && kind <= 9); sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        return sceValue, scealgorithm.Precondition
    }
    if sceCond1 := !(func() bool { if kind == 4 { return status >= 200 && status <= 599 }; return status == 0 }()); sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        return sceValue, scealgorithm.Precondition
    }
    var outcome uint8 = 2
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    if sceCond1 := kind == 4 && status < 500; sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        outcome = func() uint8 { if status < 300 || status == 404 || status == 410 { return 0 }; return 1 }();
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
    }
    {
        var sceReturned uint8 = outcome
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        return sceReturned, nil
    }
}
