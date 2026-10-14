// SCE-GENERATED — DO NOT EDIT
// source-hash: 63d8a2a082a19d1554025076e46fc78cfae3009b81ba27d2c900116fefc6df03
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
