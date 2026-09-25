// SCE-MAP: algorithm_checked_arith:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `algorithm_checked_arith`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package algorithm_checked_arith

import (
	scealgorithm "github.com/newmassrael/sce-forge-runtime/algorithm"
)

func AlgorithmCheckedArith(a int32, b int32, op uint8) (sceValue int32, sceErr error) {
    // SCE_FORGE.md §3.4.1: each checked operation records a failure here, and
    // the statement around it returns it before the next statement runs.
    var sceFailure scealgorithm.Failure
    var r int32 = 0
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    if sceCond1 := op == 0; sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        r = scealgorithm.AddInt32(&sceFailure, a, b);
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
    }
    if sceCond1 := op == 1; sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        r = scealgorithm.SubInt32(&sceFailure, a, b);
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
    }
    if sceCond1 := op == 2; sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        r = scealgorithm.MulInt32(&sceFailure, a, b);
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
    }
    if sceCond1 := op == 3; sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        r = scealgorithm.DivInt32(&sceFailure, a, b);
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
    }
    if sceCond1 := op == 4; sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        r = scealgorithm.RemInt32(&sceFailure, a, b);
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
    }
    if sceCond1 := op == 5; sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        r = scealgorithm.NegInt32(&sceFailure, a);
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
    }
    if sceCond1 := op == 6; sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        r = int32(scealgorithm.SubUint8(&sceFailure, op, 7));
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
    }
    {
        var sceReturned int32 = r
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        return sceReturned, nil
    }
}
