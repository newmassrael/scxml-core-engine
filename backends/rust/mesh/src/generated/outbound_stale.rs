// SCE-GENERATED — DO NOT EDIT
// source-hash: 0ffd4f5aaee672eb45b262966e33f5e18902327fd6170e44dc26c067a1ddada4
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
