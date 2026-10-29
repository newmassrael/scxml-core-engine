// SCE-GENERATED — DO NOT EDIT
// source-hash: 75c0c87af90d82dec15f516a721dd486d47e1280589aefe06f15efdef5f4f850
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
