// SCE-GENERATED — DO NOT EDIT
// source-hash: ab119d19c373fb9e83e74bd30f74186a9e2f87ef70ba045b5c5ab8bb9e9d1849
#![doc = "SCE-MAP: outbound_stale.scxml:18 :: _forge_body"]
// SCE-MAP: outbound_stale.scxml:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn outbound_stale(
    enqueued_at_ms: i64,
    now: i64,
    max_age_ms: i64,
) -> Result<bool, sce_forge_runtime::algorithm::AlgorithmError> {
    let mut stale: bool = false;
    if max_age_ms > 0 {
        stale = sce_forge_runtime::algorithm::sub::<i64>(now, enqueued_at_ms)? > max_age_ms;
    }
    return Ok(stale);
}
