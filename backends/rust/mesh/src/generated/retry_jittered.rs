// SCE-GENERATED — DO NOT EDIT
// source-hash: 0ffd4f5aaee672eb45b262966e33f5e18902327fd6170e44dc26c067a1ddada4
#![doc = "SCE-MAP: retry_jittered.scxml:23 :: _forge_body"]
// SCE-MAP: retry_jittered.scxml:23 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn retry_jittered(
    base_ms: i64,
    jitter_pct: i64,
    draw: i64,
) -> Result<i64, sce_forge_runtime::algorithm::AlgorithmError> {
    if !(base_ms > 0 && jitter_pct >= 0 && jitter_pct <= 100 && draw >= 0) {
        return Err(sce_forge_runtime::algorithm::AlgorithmError::Precondition);
    }
    let delta: i64 = sce_forge_runtime::algorithm::add::<i64>(
        sce_forge_runtime::algorithm::mul::<i64>(
            sce_forge_runtime::algorithm::div::<i64>(base_ms, 100)?,
            jitter_pct,
        )?,
        sce_forge_runtime::algorithm::div::<i64>(
            sce_forge_runtime::algorithm::mul::<i64>(
                sce_forge_runtime::algorithm::rem::<i64>(base_ms, 100)?,
                jitter_pct,
            )?,
            100,
        )?,
    )?;
    let waited: i64 = sce_forge_runtime::algorithm::add::<i64>(
        sce_forge_runtime::algorithm::sub::<i64>(base_ms, delta)?,
        sce_forge_runtime::algorithm::rem::<i64>(
            draw,
            sce_forge_runtime::algorithm::add::<i64>(
                sce_forge_runtime::algorithm::mul::<i64>(2, delta)?,
                1,
            )?,
        )?,
    )?;
    return Ok(if waited < 1 { 1 } else { waited });
}
