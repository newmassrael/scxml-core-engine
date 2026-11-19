// SCE-GENERATED — DO NOT EDIT
// source-hash: 84ff81b033009528cb45ea402416f0964c32998c99c764e4cae0b42faa8f879e
// source: ordering_prune.scxml
#![doc = "SCE-MAP: ordering_prune.scxml:11 :: _forge_body"]
// SCE-MAP: ordering_prune.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

use super::ordering_slot;

use sce_portable_bytes::{CapacityExceeded, SceOwnedList};

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn ordering_prune(
    pending: &[ordering_slot::OrderingSlotPayload],
    next: u64,
    out: &mut SceOwnedList<ordering_slot::OrderingSlotPayload, 256>,
) -> Result<(), CapacityExceeded> {
    out.clear();
    out.reserve(256);
    for &slot in pending.iter() {
        if slot.seq >= next {
            out.push(slot)?;
        }
    }
    return Ok(());
}
