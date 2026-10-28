// SCE-GENERATED — DO NOT EDIT
// source-hash: b67ac92c47dabab09bc66be64b0a4b4a6acfabe17f95412102ed640a717fc04f
// SCE-MAP: algorithm_ledger_ids.scxml:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `ledger_ids`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package ledger_ids

import (
	scealgorithm "github.com/newmassrael/sce-forge-runtime/algorithm"
)

func LedgerIds(ids []uint32) (sceValue uint32, sceErr error) {
    // SCE_FORGE.md §3.4.1: each checked operation records a failure here, and
    // the statement around it returns it before the next statement runs.
    var sceFailure scealgorithm.Failure
    var sum uint32 = 0
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    for _, id := range ids {
        sum = scealgorithm.AddUint32(&sceFailure, sum, id);
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
    }
    {
        var sceReturned uint32 = sum
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        return sceReturned, nil
    }
}
