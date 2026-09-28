// SCE-GENERATED — DO NOT EDIT
// source-hash: e2a3a8b4e1357d9d4d92950925a9e4c7d6137a4246c0000110af1331c99ac086
#![doc = "SCE-MAP: ordering_gap_end.scxml:21 :: _forge_body"]
// SCE-MAP: ordering_gap_end.scxml:21 :: _forge_body

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
pub fn ordering_gap_end(
    pending: &[ordering_slot::OrderingSlotPayload],
    next: u64,
    now: i64,
    timeout_ms: i64,
) -> Result<u64, sce_forge_runtime::algorithm::AlgorithmError> {
    let mut resume: u64 = next;
    let mut first: bool = true;
    for &slot in pending.iter() {
        if first
            && slot.seq != next
            && sce_forge_runtime::algorithm::sub::<i64>(now, slot.arrived_at_ms)? >= timeout_ms
        {
            resume = slot.seq;
        }
        first = false;
    }
    return Ok(resume);
}
