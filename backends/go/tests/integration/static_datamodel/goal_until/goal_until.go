// SCE-GENERATED — DO NOT EDIT
// source-hash: 1178ba59796ca1f7c46f8569fd4bfda85c2b97db65f51728f3615d990d97dd1c
// SCE-MAP: algorithm_goal_until.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `goal_until`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package goal_until

import (
	scealgorithm "github.com/newmassrael/sce-forge-runtime/algorithm"
	"github.com/newmassrael/sce-forge-runtime/optional"
)

func GoalUntil(today int64, day optional.Of[int64]) (sceValue optional.Of[int64], sceErr error) {
    // SCE_FORGE.md §3.4.1: each checked operation records a failure here, and
    // the statement around it returns it before the next statement runs.
    var sceFailure scealgorithm.Failure
    if sceCond1 := !day.IsPresent(); sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        {
            var sceReturned optional.Of[int64] = optional.None[int64]()
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
            return sceReturned, nil
        }
    }
    {
        var sceReturned optional.Of[int64] = optional.Some[int64](scealgorithm.SubInt64(&sceFailure, day.Proven(), today))
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        return sceReturned, nil
    }
}
