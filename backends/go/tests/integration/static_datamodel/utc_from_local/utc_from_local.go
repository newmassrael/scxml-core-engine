// SCE-GENERATED — DO NOT EDIT
// source-hash: 68d77b5e0c96e361239e1512887ba2ef6d35bfab0188bf0e2afbd53ca387f3bc
// SCE-MAP: utc_from_local.scxml:30 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `utc_from_local`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package utc_from_local

import (
	scealgorithm "github.com/newmassrael/sce-forge-runtime/algorithm"
)

func UtcFromLocal(starts []int64, offsets []int32, local int64) (sceValue int64, sceErr error) {
    // SCE_FORGE.md §3.4.1: each checked operation records a failure here, and
    // the statement around it returns it before the next statement runs.
    var sceFailure scealgorithm.Failure
    if sceCond1 := !(len(starts) == len(offsets) && len(starts) > 0); sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        return sceValue, scealgorithm.Precondition
    }
    if sceCond1 := !(scealgorithm.SubInt64(&sceFailure, local, int64(scealgorithm.At(&sceFailure, offsets[:], int64(0)))) >= scealgorithm.At(&sceFailure, starts[:], int64(0))); sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        return sceValue, scealgorithm.Precondition
    }
    var n uint32 = uint32(len(starts))
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var k uint32 = 0
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    for {
        sceCond1 := k < n
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        if !sceCond1 {
            break
        }
        if sceCond2 := k > 0; sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        } else if sceCond2 {
            if sceCond3 := !(scealgorithm.AtU(&sceFailure, starts[:], uint64(k)) > scealgorithm.AtU(&sceFailure, starts[:], uint64(scealgorithm.SubUint32(&sceFailure, k, 1)))); sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            } else if sceCond3 {
                return sceValue, scealgorithm.Precondition
            }
        }
        var u int64 = scealgorithm.SubInt64(&sceFailure, local, int64(scealgorithm.AtU(&sceFailure, offsets[:], uint64(k))))
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        if sceCond2 := u >= scealgorithm.AtU(&sceFailure, starts[:], uint64(k)) && (scealgorithm.AddUint32(&sceFailure, k, 1) == n || u < scealgorithm.AtU(&sceFailure, starts[:], uint64(scealgorithm.AddUint32(&sceFailure, k, 1)))); sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        } else if sceCond2 {
            {
                var sceReturned int64 = u
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                return sceReturned, nil
            }
        }
        k = scealgorithm.AddUint32(&sceFailure, k, 1);
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
    }
    var g uint32 = 1
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    for {
        sceCond1 := g < n
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        if !sceCond1 {
            break
        }
        var before int64 = scealgorithm.SubInt64(&sceFailure, local, int64(scealgorithm.AtU(&sceFailure, offsets[:], uint64(scealgorithm.SubUint32(&sceFailure, g, 1)))))
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        if sceCond2 := before >= scealgorithm.AtU(&sceFailure, starts[:], uint64(g)) && scealgorithm.SubInt64(&sceFailure, local, int64(scealgorithm.AtU(&sceFailure, offsets[:], uint64(g)))) < scealgorithm.AtU(&sceFailure, starts[:], uint64(g)); sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        } else if sceCond2 {
            {
                var sceReturned int64 = before
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                return sceReturned, nil
            }
        }
        g = scealgorithm.AddUint32(&sceFailure, g, 1);
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
    }
    if sceCond1 := !(false); sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        return sceValue, scealgorithm.Precondition
    }
    {
        var sceReturned int64 = 0
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        return sceReturned, nil
    }
}
