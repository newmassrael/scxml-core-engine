// SCE-GENERATED — DO NOT EDIT
// source-hash: 58f5a17e479c571b0f1a95c9ced6b8f03b5d957325be03d16f15d59d99727b1b
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
