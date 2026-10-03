// SCE-GENERATED — DO NOT EDIT
// source-hash: 8fa41ba4d02ec0c660aae577c6604c3b5470da58c62da65404cc05ef8dc67bfe
// SCE-MAP: sync_upload_outcome.scxml:24 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `sync_upload_outcome`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package sync_upload_outcome

import (
	scealgorithm "github.com/newmassrael/sce-forge-runtime/algorithm"
)

func SyncUploadOutcome(create bool, status int32, davError bool) (sceValue uint8, sceErr error) {
    // SCE_FORGE.md §3.4.1: each checked operation records a failure here, and
    // the statement around it returns it before the next statement runs.
    var sceFailure scealgorithm.Failure
    if sceCond1 := !(status >= 200 && status <= 599); sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        return sceValue, scealgorithm.Precondition
    }
    var outcome uint8 = 2
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    if sceCond1 := status < 300; sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        outcome = 0;
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
    }
    if sceCond1 := status == 403 || status == 412 || status == 409 && davError; sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        outcome = 1;
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
    }
    if sceCond1 := !create && (status == 404 || status == 410 || status == 409); sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        outcome = 1;
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
