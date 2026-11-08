// SCE-GENERATED — DO NOT EDIT
// source-hash: ea764ab5524bc7a7e7d4553640b3fb26547d13f0d2053c0f5e7214161efbc6d0
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
    public static func goalUntil(_ today: Int64, _ day: Int64?) throws -> Int64? {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        do {
            let sceCond2 = day == nil
            try sceFailure.throwIfFailed()
            if sceCond2 {
                do {
                    let sceReturned: Int64? = nil
                    try sceFailure.throwIfFailed()
                    return sceReturned
                }
            }
        }
        do {
            let sceReturned: Int64? = Checked.sub(sceFailure, SceOptional.proven(day), today)
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
