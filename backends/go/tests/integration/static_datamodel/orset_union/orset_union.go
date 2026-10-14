// SCE-GENERATED — DO NOT EDIT
// source-hash: c431fa9e45990e8b729e69a4bf81ba13f9cc4e8066ddecbd5ff6346ac56dd5f3
// SCE-MAP: orset_union.scxml:23 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `orset_union`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package orset_union

import (
	scealgorithm "github.com/newmassrael/sce-forge-runtime/algorithm"
	sce_orset_entry "github.com/newmassrael/sce-go-tests/integration/static_datamodel/orset_entry"
)

func OrsetUnion(a []sce_orset_entry.OrsetEntryPayload, b []sce_orset_entry.OrsetEntryPayload) (sceValue []sce_orset_entry.OrsetEntryPayload, sceErr error) {
    // SCE_FORGE.md §3.4.1: each checked operation records a failure here, and
    // the statement around it returns it before the next statement runs.
    var sceFailure scealgorithm.Failure
    out := make([]sce_orset_entry.OrsetEntryPayload, 0, 256)
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    for _, x := range a {
        out = append(out, x)
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
    }
    for _, y := range b {
        var held bool = false
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        for _, z := range a {
            if sceCond3 := z.Element == y.Element && z.WallTime == y.WallTime && z.Counter == y.Counter && z.NodeId == y.NodeId; sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            } else if sceCond3 {
                held = true;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
            }
        }
        if sceCond2 := !held; sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        } else if sceCond2 {
            out = append(out, y)
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
