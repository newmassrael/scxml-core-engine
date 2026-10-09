// SCE-GENERATED — DO NOT EDIT
// source-hash: cb41954d893211ff980559eb7566d5cfca26a8d312a941a8b0426661b4fb8dcd
// SCE-MAP: algorithm_day_repeat.scxml:13 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `day_repeat`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package day_repeat

import (
	scealgorithm "github.com/newmassrael/sce-forge-runtime/algorithm"
)

func DayRepeat(value uint8, count uint8) (sceValue []uint8, sceErr error) {
    // SCE_FORGE.md §3.4.1: each checked operation records a failure here, and
    // the statement around it returns it before the next statement runs.
    var sceFailure scealgorithm.Failure
    out := make([]uint8, 0, 4)
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
        out = append(out, uint8(value))
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
