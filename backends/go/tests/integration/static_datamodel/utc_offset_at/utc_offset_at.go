// SCE-GENERATED — DO NOT EDIT
// source-hash: 68d77b5e0c96e361239e1512887ba2ef6d35bfab0188bf0e2afbd53ca387f3bc
// SCE-MAP: utc_offset_at.scxml:25 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `utc_offset_at`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package utc_offset_at

import (
	scealgorithm "github.com/newmassrael/sce-forge-runtime/algorithm"
)

func UtcOffsetAt(starts []int64, offsets []int32, t int64) (sceValue int32, sceErr error) {
    // SCE_FORGE.md §3.4.1: each checked operation records a failure here, and
    // the statement around it returns it before the next statement runs.
    var sceFailure scealgorithm.Failure
    if sceCond1 := !(len(starts) == len(offsets) && len(starts) > 0); sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        return sceValue, scealgorithm.Precondition
    }
    if sceCond1 := !(t >= scealgorithm.At(&sceFailure, starts[:], int64(0))); sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        return sceValue, scealgorithm.Precondition
    }
    var k uint32 = 0
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var i uint32 = 1
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    for {
        sceCond1 := i < uint32(len(starts))
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        if !sceCond1 {
            break
        }
        if sceCond2 := !(scealgorithm.AtU(&sceFailure, starts[:], uint64(i)) > scealgorithm.AtU(&sceFailure, starts[:], uint64(scealgorithm.SubUint32(&sceFailure, i, 1)))); sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        } else if sceCond2 {
            return sceValue, scealgorithm.Precondition
        }
        if sceCond2 := scealgorithm.AtU(&sceFailure, starts[:], uint64(i)) <= t; sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        } else if sceCond2 {
            k = i;
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
        }
        i = scealgorithm.AddUint32(&sceFailure, i, 1);
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
    }
    {
        var sceReturned int32 = scealgorithm.AtU(&sceFailure, offsets[:], uint64(k))
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        return sceReturned, nil
    }
}
