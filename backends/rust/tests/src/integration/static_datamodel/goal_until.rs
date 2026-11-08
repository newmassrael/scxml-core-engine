// SCE-GENERATED — DO NOT EDIT
// source-hash: a0d514a5bb9842543c3a83f1c232fcee900a165f2a3ed19fb29e42d249c8aa22
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
