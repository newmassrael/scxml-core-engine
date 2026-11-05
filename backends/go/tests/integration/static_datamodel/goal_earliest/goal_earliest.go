// SCE-GENERATED — DO NOT EDIT
// source-hash: dcc96ea8c5321d36e69f038b98535f0793d14bdbecc7d72d9cf5831d9f96378c
// SCE-MAP: algorithm_goal_earliest.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `goal_earliest`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package goal_earliest

import (
	"github.com/newmassrael/sce-forge-runtime/optional"
	sce_schema_goal "github.com/newmassrael/sce-go-tests/integration/static_datamodel/schema_goal"
)

func GoalEarliest(goals []sce_schema_goal.SchemaGoalPayload) optional.Of[int64] {
    var best optional.Of[int64] = optional.None[int64]()
    for _, g := range goals {
        if (g.Day.IsPresent() && (!best.IsPresent() || g.Day.Proven() < best.Proven())) {
            best = g.Day;
        }
    }
    return best
}
