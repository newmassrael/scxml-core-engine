// SCE-GENERATED — DO NOT EDIT
// source-hash: 02a1f302f8985c8979205027eb5b236990d7a79fc8e74a6197aa1d9c5b50cead
// SCE-MAP: algorithm_goal_day_sum.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `GoalDaySum`, a file of the
// module that calls it: a call is qualified by the enum
// (`GoalDaySum.goalDaySum(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum GoalDaySum {
    public static func goalDaySum(_ goals: [SchemaGoalPayload], _ instead: Int64) throws(AlgorithmError) -> Int64 {
        var sum: Int64 = 0
        for g in goals {
            sum = (try Checked.add(sum, g.day ?? instead))
        }
        return sum
    }
}
