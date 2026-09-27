// SCE-GENERATED — DO NOT EDIT
// source-hash: f6148b213f9ed141b381c5b54aabc8b1a585f1b96413497e91bf4f50d5cda1ce
#![doc = "SCE-MAP: ordering_hold.scxml:16 :: _forge_body"]
// SCE-MAP: ordering_hold.scxml:16 :: _forge_body

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
pub fn ordering_hold(
    pending: &[ordering_slot::OrderingSlotPayload],
    seq: u64,
    now: i64,
) -> Result<SceOwnedList<ordering_slot::OrderingSlotPayload, 256>, CapacityExceeded> {
    let arrived = ordering_slot::OrderingSlotPayload {
        seq: seq,
        arrived_at_ms: now,
    };
    let mut out: SceOwnedList<ordering_slot::OrderingSlotPayload, 256> = SceOwnedList::new();
    let mut placed: bool = false;
    for &slot in pending.iter() {
        if slot.seq == seq {
            placed = true;
        }
        if !placed && seq < slot.seq {
            out.push(arrived)?;
            placed = true;
        }
        out.push(slot)?;
    }
    if !placed {
        out.push(arrived)?;
    }
    return Ok(out);
}
