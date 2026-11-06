// SCE-GENERATED — DO NOT EDIT
// source-hash: 9aa28cb8a6152625fb0eb1560e8ab3014da5864745e85b6f3b6e37672c04dcf6
#![doc = "SCE-MAP: algorithm_goal_day_sum.scxml:8 :: _forge_body"]
// SCE-MAP: algorithm_goal_day_sum.scxml:8 :: _forge_body

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
pub fn goal_day_sum(
    goals: &[schema_goal::SchemaGoalPayload],
    instead: i64,
) -> Result<i64, sce_forge_runtime::algorithm::AlgorithmError> {
    let mut sum: i64 = 0;
    for &g in goals.iter() {
        sum = sce_forge_runtime::algorithm::add::<i64>(
            sum,
            match g.day {
                Some(sce_present) => sce_present,
                None => instead,
            },
        )?;
    }
    return Ok(sum);
}
