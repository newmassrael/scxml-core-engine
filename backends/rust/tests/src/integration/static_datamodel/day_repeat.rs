// SCE-GENERATED — DO NOT EDIT
// source-hash: 552d5eb22ef933056085dce88fe5367b344dcf477c9ae66190ff1867ab30429e
#![doc = "SCE-MAP: algorithm_day_repeat.scxml:13 :: _forge_body"]
// SCE-MAP: algorithm_day_repeat.scxml:13 :: _forge_body

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
pub fn day_repeat(
    value: u8,
    count: u8,
) -> Result<SceOwnedList<u8, 4>, sce_forge_runtime::algorithm::AlgorithmError> {
    let mut out: SceOwnedList<u8, 4> = SceOwnedList::new();
    let mut i: u8 = 0;
    while i < count {
        out.push(value)?;
        i = sce_forge_runtime::algorithm::add::<u8>(i, 1)?;
    }
    return Ok(out);
}
