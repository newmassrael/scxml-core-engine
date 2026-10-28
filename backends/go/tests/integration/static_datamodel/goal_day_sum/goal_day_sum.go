// SCE-GENERATED — DO NOT EDIT
// source-hash: d11fb6db69c27fed25f47a2f22a5bf7fb78da524d08f0c299e9cc553d32a6375
// SCE-MAP: algorithm_goal_day_sum.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `goal_day_sum`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package goal_day_sum

import (
	scealgorithm "github.com/newmassrael/sce-forge-runtime/algorithm"
	"github.com/newmassrael/sce-forge-runtime/optional"
	sce_schema_goal "github.com/newmassrael/sce-go-tests/integration/static_datamodel/schema_goal"
)

func GoalDaySum(goals []sce_schema_goal.SchemaGoalPayload, instead int64) (sceValue int64, sceErr error) {
    // SCE_FORGE.md §3.4.1: each checked operation records a failure here, and
    // the statement around it returns it before the next statement runs.
    var sceFailure scealgorithm.Failure
    var sum int64 = 0
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    for _, g := range goals {
        sum = scealgorithm.AddInt64(&sceFailure, sum, optional.Otherwise(g.Day, func() int64 { return instead }));
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
    }
    {
        var sceReturned int64 = sum
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        return sceReturned, nil
    }
}
