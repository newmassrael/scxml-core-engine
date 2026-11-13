// SCE-GENERATED — DO NOT EDIT
// source-hash: 138d54bef1f6b9b0e78600cb129e88342af00344f9fa58b6a90edf9adcaeb40d
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
