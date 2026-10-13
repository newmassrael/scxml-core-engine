// SCE-GENERATED — DO NOT EDIT
// source-hash: 5d4270ae4c884e43e7a12f9a58bd5d012629cb9a3aef7240aa93f930de85d6c6
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
