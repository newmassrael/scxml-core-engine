// SCE-GENERATED — DO NOT EDIT
// source-hash: fbe1b9e6d3d44a234eeb738162c670f9c393ccc06557a4f68a8f54a0834b710c
// SCE-MAP: register_values.scxml:21 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `register_values`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package register_values

import (
	scealgorithm "github.com/newmassrael/sce-forge-runtime/algorithm"
	sce_mv_value "github.com/newmassrael/sce-go-tests/integration/static_datamodel/mv_value"
)

func RegisterValues(values []sce_mv_value.MvValuePayload, id uint64, field uint64) (sceValue []sce_mv_value.MvValuePayload, sceErr error) {
    // SCE_FORGE.md §3.4.1: each checked operation records a failure here, and
    // the statement around it returns it before the next statement runs.
    var sceFailure scealgorithm.Failure
    out := make([]sce_mv_value.MvValuePayload, 0, 512)
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var started bool = false
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var lastWall int64 = 0
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var lastCounter uint32 = 0
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var lastNode uint64 = 0
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var more bool = true
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    for {
        sceCond1 := more
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        if !sceCond1 {
            break
        }
        var chosen bool = false
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var bestWall int64 = 0
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var bestCounter uint32 = 0
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var bestNode uint64 = 0
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        for _, v := range values {
            if sceCond3 := v.Id == id && v.Field == field && (!started || v.WallTime > lastWall || v.WallTime == lastWall && (v.Counter > lastCounter || v.Counter == lastCounter && v.NodeId > lastNode)) && (!chosen || v.WallTime < bestWall || v.WallTime == bestWall && (v.Counter < bestCounter || v.Counter == bestCounter && v.NodeId < bestNode)); sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            } else if sceCond3 {
                chosen = true;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                bestWall = v.WallTime;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                bestCounter = v.Counter;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                bestNode = v.NodeId;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
            }
        }
        if sceCond2 := chosen; sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        } else if sceCond2 {
            for _, w := range values {
                if sceCond4 := w.Id == id && w.Field == field && w.WallTime == bestWall && w.Counter == bestCounter && w.NodeId == bestNode; sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                } else if sceCond4 {
                    out = append(out, w)
                    if sceFailure.Failed() {
                        return sceValue, sceFailure.Err()
                    }
                }
            }
            started = true;
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
            lastWall = bestWall;
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
            lastCounter = bestCounter;
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
            lastNode = bestNode;
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
        } else {
            more = false;
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
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
