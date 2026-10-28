// SCE-GENERATED — DO NOT EDIT
// source-hash: 58f5a17e479c571b0f1a95c9ced6b8f03b5d957325be03d16f15d59d99727b1b
#![doc = "SCE-MAP: ordering_drain.scxml:15 :: _forge_body"]
// SCE-MAP: ordering_drain.scxml:15 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

use super::ordering_slot;

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn ordering_drain(
    pending: &[ordering_slot::OrderingSlotPayload],
    start: u64,
) -> Result<u64, sce_forge_runtime::algorithm::AlgorithmError> {
    let mut next: u64 = start;
    for &slot in pending.iter() {
        if slot.seq == next {
            next = sce_forge_runtime::algorithm::add::<u64>(next, 1)?;
        }
    }
    return Ok(next);
}
