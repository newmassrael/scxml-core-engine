// SCE-GENERATED — DO NOT EDIT
// source-hash: ab119d19c373fb9e83e74bd30f74186a9e2f87ef70ba045b5c5ab8bb9e9d1849
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
) -> Result<SceOwnedList<ordering_slot::OrderingSlotPayload, 256>, CapacityExceeded> {
    let mut out: SceOwnedList<ordering_slot::OrderingSlotPayload, 256> = SceOwnedList::new();
    for &slot in pending.iter() {
        if slot.seq >= next {
            out.push(slot)?;
        }
    }
    return Ok(out);
}
