// SCE-GENERATED — DO NOT EDIT
// source-hash: 1178ba59796ca1f7c46f8569fd4bfda85c2b97db65f51728f3615d990d97dd1c
// SCE-MAP: algorithm_goal_earliest.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `GoalEarliest`, a file of the
// module that calls it: a call is qualified by the enum
// (`GoalEarliest.goalEarliest(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum GoalEarliest {
    public static func goalEarliest(_ goals: [SchemaGoalPayload]) -> Int64? {
        var best: Int64? = nil
        for g in goals {
            if g.day != nil && (best == nil || SceOptional.proven(g.day) < SceOptional.proven(best)) {
                best = g.day
            }
        }
        return best
    }
}
