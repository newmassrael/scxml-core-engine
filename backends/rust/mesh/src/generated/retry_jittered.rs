// SCE-GENERATED — DO NOT EDIT
// source-hash: ab119d19c373fb9e83e74bd30f74186a9e2f87ef70ba045b5c5ab8bb9e9d1849
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
