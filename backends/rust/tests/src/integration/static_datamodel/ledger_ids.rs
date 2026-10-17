// SCE-GENERATED — DO NOT EDIT
// source-hash: 38a2b8ab9943c3f76e7281d40f9dae3ea2effb54aa8b7ab5baa1ac0eaba97f5b
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
