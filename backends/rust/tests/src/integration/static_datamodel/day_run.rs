// SCE-GENERATED — DO NOT EDIT
// source-hash: 5d22e54cdaa26fa7d37f9212c73eac0589e479e7d084e83b6ac57dae4eb27663
#![doc = "SCE-MAP: algorithm_day_run.scxml:9 :: _forge_body"]
// SCE-MAP: algorithm_day_run.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

use sce_portable_bytes::SceOwnedList;

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn day_run(
    first: u8,
    count: u8,
) -> Result<SceOwnedList<u8, 8>, sce_forge_runtime::algorithm::AlgorithmError> {
    let mut out: SceOwnedList<u8, 8> = SceOwnedList::new();
    let mut i: u8 = 0;
    while i < count {
        out.push(sce_forge_runtime::algorithm::add::<u8>(first, i)?)?;
        i = sce_forge_runtime::algorithm::add::<u8>(i, 1)?;
    }
    return Ok(out);
}
