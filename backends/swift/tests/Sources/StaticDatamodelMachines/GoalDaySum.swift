// SCE-GENERATED — DO NOT EDIT
// source-hash: d05ce240c5f3b48b3c83b6bee1ba58feef40d57c310e9a29ef5279604bbaaa42
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
    public static func goalDaySum(_ goals: [SchemaGoalPayload], _ instead: Int64) throws -> Int64 {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        var sum: Int64 = 0
        try sceFailure.throwIfFailed()
        for g in goals {
            sum = Checked.add(sceFailure, sum, g.day ?? instead)
            try sceFailure.throwIfFailed()
        }
        do {
            let sceReturned: Int64 = sum
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
