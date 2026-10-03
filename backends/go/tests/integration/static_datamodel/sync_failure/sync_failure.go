// SCE-GENERATED — DO NOT EDIT
// source-hash: 8fa41ba4d02ec0c660aae577c6604c3b5470da58c62da65404cc05ef8dc67bfe
// SCE-MAP: sync_failure.scxml:30 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `sync_failure`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package sync_failure

import (
	scealgorithm "github.com/newmassrael/sce-forge-runtime/algorithm"
)

func SyncFailure(kind uint8, status int32) (sceValue uint8, sceErr error) {
    // SCE_FORGE.md §3.4.1: each checked operation records a failure here, and
    // the statement around it returns it before the next statement runs.
    var sceFailure scealgorithm.Failure
    if sceCond1 := !(kind >= 1 && kind <= 9); sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        return sceValue, scealgorithm.Precondition
    }
    if sceCond1 := !(func() bool { if kind == 4 { return status >= 300 && status <= 599 }; return status == 0 }()); sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        return sceValue, scealgorithm.Precondition
    }
    var action uint8 = 2
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    if sceCond1 := kind == 8 || kind == 1 || kind == 2; sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        action = 1;
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
    }
    if sceCond1 := kind == 7; sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        action = 4;
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
    }
    if sceCond1 := kind == 3; sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        action = 0;
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
    }
    if sceCond1 := kind == 4 && status == 401; sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        action = 3;
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
    }
    if sceCond1 := kind == 4 && (status == 503 || status == 502); sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        action = 1;
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
    }
    {
        var sceReturned uint8 = action
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        return sceReturned, nil
    }
}
