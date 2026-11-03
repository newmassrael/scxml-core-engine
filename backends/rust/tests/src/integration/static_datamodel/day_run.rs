// SCE-GENERATED — DO NOT EDIT
// source-hash: 6fe63bbaa76604f09a48d24750e75e0a784e90fcb598cdca75c3ac643915bb8d
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
    out: &mut SceOwnedList<u8, 8>,
) -> Result<(), sce_forge_runtime::algorithm::AlgorithmError> {
    out.clear();
    let mut i: u8 = 0;
    while i < count {
        out.push(sce_forge_runtime::algorithm::add::<u8>(first, i)?)?;
        i = sce_forge_runtime::algorithm::add::<u8>(i, 1)?;
    }
    return Ok(());
}
