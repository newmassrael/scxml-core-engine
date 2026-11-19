// SCE-GENERATED — DO NOT EDIT
// source-hash: 6d11eebd31ae951e37d15993059cc1cbdb6600ff171bb0a1bb0cfbc30f6f49c1
// source: algorithm_goal_until.scxml
// SCE-MAP: algorithm_goal_until.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `GoalUntil`, a file of the
// module that calls it: a call is qualified by the enum
// (`GoalUntil.goalUntil(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum GoalUntil {
    public static func goalUntil(_ today: Int64, _ day: Int64?) throws(AlgorithmError) -> Int64? {
        if day == nil {
            return nil
        }
        return (try Checked.sub(SceOptional.proven(day), today))
    }
}
