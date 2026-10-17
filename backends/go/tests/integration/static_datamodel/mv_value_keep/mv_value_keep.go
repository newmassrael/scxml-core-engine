// SCE-GENERATED — DO NOT EDIT
// source-hash: 9ad3991e659415a87e89be4422403fddaea9105e2a522b988eeacca0944ca255
// SCE-MAP: mv_value_keep.scxml:22 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `mv_value_keep`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package mv_value_keep

import (
	scealgorithm "github.com/newmassrael/sce-forge-runtime/algorithm"
	sce_mv_value "github.com/newmassrael/sce-go-tests/integration/static_datamodel/mv_value"
)

func MvValueKeep(values []sce_mv_value.MvValuePayload, held sce_mv_value.MvValuePayload) (sceValue []sce_mv_value.MvValuePayload, sceErr error) {
    // SCE_FORGE.md §3.4.1: each checked operation records a failure here, and
    // the statement around it returns it before the next statement runs.
    var sceFailure scealgorithm.Failure
    out := make([]sce_mv_value.MvValuePayload, 0, 512)
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var there bool = false
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    for _, v := range values {
        if sceCond2 := v.Id == held.Id && v.Field == held.Field && v.WallTime == held.WallTime && v.Counter == held.Counter && v.NodeId == held.NodeId; sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        } else if sceCond2 {
            there = true;
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
        }
        out = append(out, v)
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
    }
    if sceCond1 := !there; sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        out = append(out, held)
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
    }
    {
        var sceReturned []sce_mv_value.MvValuePayload = out
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        return sceReturned, nil
    }
}
