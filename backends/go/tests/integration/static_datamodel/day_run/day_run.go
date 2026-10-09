// SCE-GENERATED — DO NOT EDIT
// source-hash: ee37533f857b01b43223ccdf57f0449c0b326ab00d940ee5b3e09fb6a557b07f
// SCE-MAP: algorithm_day_run.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `day_run`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package day_run

import (
	scealgorithm "github.com/newmassrael/sce-forge-runtime/algorithm"
)

func DayRun(first uint8, count uint8) (sceValue []uint8, sceErr error) {
    // SCE_FORGE.md §3.4.1: each checked operation records a failure here, and
    // the statement around it returns it before the next statement runs.
    var sceFailure scealgorithm.Failure
    out := make([]uint8, 0, 8)
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var i uint8 = 0
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    for {
        sceCond1 := i < count
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        if !sceCond1 {
            break
        }
        out = append(out, uint8(scealgorithm.AddUint8(&sceFailure, first, i)))
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        i = scealgorithm.AddUint8(&sceFailure, i, 1);
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
    }
    {
        var sceReturned []uint8 = out
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        return sceReturned, nil
    }
}
