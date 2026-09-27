// SCE-GENERATED — DO NOT EDIT
// source-hash: ab119d19c373fb9e83e74bd30f74186a9e2f87ef70ba045b5c5ab8bb9e9d1849
#![doc = "SCE-MAP: retry_next_backoff.scxml:19 :: _forge_body"]
// SCE-MAP: retry_next_backoff.scxml:19 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn retry_next_backoff(
    prev_ms: i64,
    multiplier: f64,
    max_ms: i64,
) -> Result<i64, sce_forge_runtime::algorithm::AlgorithmError> {
    if !(multiplier >= 1.0 && prev_ms > 0 && max_ms >= prev_ms) {
        return Err(sce_forge_runtime::algorithm::AlgorithmError::Precondition);
    }
    let grown: f64 = prev_ms as f64 * multiplier;
    let mut next: i64 = max_ms;
    if grown < max_ms as f64 {
        next = (grown).floor() as i64;
    }
    return Ok(next);
}
