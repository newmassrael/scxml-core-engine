// SCE-GENERATED — DO NOT EDIT
// source-hash: 418438a744050ac9cd6cb0691018fdb0ef433f86cf92fa1a16f148307512f21e
#![doc = "SCE-MAP: retry_exhausted.scxml:17 :: _forge_body"]
// SCE-MAP: retry_exhausted.scxml:17 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn retry_exhausted(
    attempts: u32,
    max_retries: u32,
    retryable: bool,
) -> Result<bool, sce_forge_runtime::algorithm::AlgorithmError> {
    if !(max_retries > 0) {
        return Err(sce_forge_runtime::algorithm::AlgorithmError::Precondition);
    }
    return Ok(!retryable || attempts > max_retries);
}
