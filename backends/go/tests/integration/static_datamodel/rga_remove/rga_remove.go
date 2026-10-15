// SCE-GENERATED — DO NOT EDIT
// source-hash: 58e52c40de99a046030dfe178fb69ffa932b0a6a8146307d7430d1e6de80a470
// SCE-MAP: rga_remove.scxml:20 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `rga_remove`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package rga_remove

import (
	scealgorithm "github.com/newmassrael/sce-forge-runtime/algorithm"
	sce_rga_gone "github.com/newmassrael/sce-go-tests/integration/static_datamodel/rga_gone"
)

func RgaRemove(gone []sce_rga_gone.RgaGonePayload, removal sce_rga_gone.RgaGonePayload) (sceValue []sce_rga_gone.RgaGonePayload, sceErr error) {
    // SCE_FORGE.md §3.4.1: each checked operation records a failure here, and
    // the statement around it returns it before the next statement runs.
    var sceFailure scealgorithm.Failure
    out := make([]sce_rga_gone.RgaGonePayload, 0, 256)
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var held bool = false
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    for _, g := range gone {
        out = append(out, g)
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        if sceCond2 := g.Key == removal.Key; sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        } else if sceCond2 {
            held = true;
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
        }
    }
    if sceCond1 := !held; sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        out = append(out, removal)
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
    }
    {
        var sceReturned []sce_rga_gone.RgaGonePayload = out
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        return sceReturned, nil
    }
}
