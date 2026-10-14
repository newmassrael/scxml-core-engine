// SCE-GENERATED — DO NOT EDIT
// source-hash: 0022d2f7097a4a1eb0e7ba717e1da8948cc1ba80ab5d45ced6c3b314777dd2db
// SCE-MAP: orset_live.scxml:31 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `orset_live`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package orset_live

import (
	scealgorithm "github.com/newmassrael/sce-forge-runtime/algorithm"
	sce_orset_entry "github.com/newmassrael/sce-go-tests/integration/static_datamodel/orset_entry"
)

func OrsetLive(adds []sce_orset_entry.OrsetEntryPayload, tombstones []sce_orset_entry.OrsetEntryPayload) (sceValue []sce_orset_entry.OrsetEntryPayload, sceErr error) {
    // SCE_FORGE.md §3.4.1: each checked operation records a failure here, and
    // the statement around it returns it before the next statement runs.
    var sceFailure scealgorithm.Failure
    out := make([]sce_orset_entry.OrsetEntryPayload, 0, 256)
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    for _, x := range adds {
        var removed bool = false
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        for _, t := range tombstones {
            if sceCond3 := t.Element == x.Element && t.WallTime == x.WallTime && t.Counter == x.Counter && t.NodeId == x.NodeId; sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            } else if sceCond3 {
                removed = true;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
            }
        }
        if sceCond2 := !removed; sceFailure.Failed() {
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
