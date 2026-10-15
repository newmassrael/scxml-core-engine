// SCE-GENERATED — DO NOT EDIT
// source-hash: a45b1edeebf5addf07d55990d33be593d588441249c6330a80971f056631ac90
// SCE-MAP: orset_observed.scxml:15 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `orset_observed`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package orset_observed

import (
	scealgorithm "github.com/newmassrael/sce-forge-runtime/algorithm"
	sce_orset_entry "github.com/newmassrael/sce-go-tests/integration/static_datamodel/orset_entry"
)

func OrsetObserved(live []sce_orset_entry.OrsetEntryPayload, element uint64) (sceValue []sce_orset_entry.OrsetEntryPayload, sceErr error) {
    // SCE_FORGE.md §3.4.1: each checked operation records a failure here, and
    // the statement around it returns it before the next statement runs.
    var sceFailure scealgorithm.Failure
    out := make([]sce_orset_entry.OrsetEntryPayload, 0, 256)
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    for _, x := range live {
        if sceCond2 := x.Element == element; sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        } else if sceCond2 {
            out = append(out, x)
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
        }
    }
    {
        var sceReturned []sce_orset_entry.OrsetEntryPayload = out
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        return sceReturned, nil
    }
}
