// SCE-GENERATED — DO NOT EDIT
// source-hash: 047dc621dfd01b6348575becd26162c58fa20fa7680f076526766f91e97d2c5f
#![doc = "SCE-MAP: algorithm_goal_undated.scxml:6 :: _forge_body"]
// SCE-MAP: algorithm_goal_undated.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

use super::schema_goal;

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn goal_undated(
    goals: &[schema_goal::SchemaGoalPayload],
) -> Result<u32, sce_forge_runtime::algorithm::AlgorithmError> {
    let mut count: u32 = 0;
    for &g in goals.iter() {
        if g.day.is_none() {
            count = sce_forge_runtime::algorithm::add::<u32>(count, 1)?;
        }
    }
    return Ok(count);
}
