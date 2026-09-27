// SCE-GENERATED — DO NOT EDIT
// source-hash: 418438a744050ac9cd6cb0691018fdb0ef433f86cf92fa1a16f148307512f21e
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
