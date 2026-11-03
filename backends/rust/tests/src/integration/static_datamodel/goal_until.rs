// SCE-GENERATED — DO NOT EDIT
// source-hash: ef94c112b4026dfab705629195b17ee265cdc572de6e8fceff328262c2af0613
#![doc = "SCE-MAP: algorithm_goal_until.scxml:9 :: _forge_body"]
// SCE-MAP: algorithm_goal_until.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn goal_until(
    today: i64,
    day: Option<i64>,
) -> Result<Option<i64>, sce_forge_runtime::algorithm::AlgorithmError> {
    if day.is_none() {
        return Ok(None);
    }
    return Ok(Some(sce_forge_runtime::algorithm::sub::<i64>(
        sce_forge_runtime::optional::proven(day),
        today,
    )?));
}
