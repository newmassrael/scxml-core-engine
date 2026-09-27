// SCE-GENERATED — DO NOT EDIT
// source-hash: f6148b213f9ed141b381c5b54aabc8b1a585f1b96413497e91bf4f50d5cda1ce
#![doc = "SCE-MAP: dedup_holds.scxml:15 :: _forge_body"]
// SCE-MAP: dedup_holds.scxml:15 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

use super::envelope_id;

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn dedup_holds(
    window: &[envelope_id::EnvelopeIdPayload],
    id: envelope_id::EnvelopeIdPayload,
) -> bool {
    let mut held: bool = false;
    for &x in window.iter() {
        if x.hi == id.hi && x.lo == id.lo {
            held = true;
        }
    }
    return held;
}
