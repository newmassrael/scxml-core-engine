// SCE-GENERATED — DO NOT EDIT
// source-hash: b2921c03007062914a1e69cc4069c6b54c8d66f97313a50f3246eed3d071c790
#![doc = "SCE-MAP: algorithm_ledger_ids.scxml:5 :: _forge_body"]
// SCE-MAP: algorithm_ledger_ids.scxml:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn ledger_ids(ids: &[u32]) -> Result<u32, sce_forge_runtime::algorithm::AlgorithmError> {
    let mut sum: u32 = 0;
    for &id in ids.iter() {
        sum = sce_forge_runtime::algorithm::add::<u32>(sum, id)?;
    }
    return Ok(sum);
}
