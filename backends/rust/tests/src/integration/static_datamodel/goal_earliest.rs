// SCE-GENERATED — DO NOT EDIT
// source-hash: 9aa28cb8a6152625fb0eb1560e8ab3014da5864745e85b6f3b6e37672c04dcf6
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
