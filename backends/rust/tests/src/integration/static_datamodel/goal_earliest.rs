// SCE-GENERATED — DO NOT EDIT
// source-hash: ea85fee4970d20335e2c3e2f4a0cd89a9f4f0a21d54f6dae4cb7a7218e2be4b5
#![doc = "SCE-MAP: algorithm_goal_earliest.scxml:11 :: _forge_body"]
// SCE-MAP: algorithm_goal_earliest.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

use super::schema_goal;

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn goal_earliest(goals: &[schema_goal::SchemaGoalPayload]) -> Option<i64> {
    let mut best: Option<i64> = None;
    for &g in goals.iter() {
        if g.day.is_some()
            && (best.is_none()
                || sce_forge_runtime::optional::proven(g.day)
                    < sce_forge_runtime::optional::proven(best))
        {
            best = g.day;
        }
    }
    return best;
}
