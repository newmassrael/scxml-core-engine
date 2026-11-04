// SCE-GENERATED — DO NOT EDIT
// source-hash: ea85fee4970d20335e2c3e2f4a0cd89a9f4f0a21d54f6dae4cb7a7218e2be4b5
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
    out: &mut SceOwnedList<u8, 4>,
) -> Result<(), sce_forge_runtime::algorithm::AlgorithmError> {
    out.clear();
    let mut i: u8 = 0;
    while i < count {
        out.push(value)?;
        i = sce_forge_runtime::algorithm::add::<u8>(i, 1)?;
    }
    return Ok(());
}
