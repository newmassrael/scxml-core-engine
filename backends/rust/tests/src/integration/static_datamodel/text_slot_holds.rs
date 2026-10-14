// SCE-GENERATED — DO NOT EDIT
// source-hash: f7347a8b98455a65e31ae2b3af12ef65d68a284242bd71a8102ab373827910fd
#![doc = "SCE-MAP: text_slot_holds.scxml:14 :: _forge_body"]
// SCE-MAP: text_slot_holds.scxml:14 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

use super::text_slot;

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn text_slot_holds(
    slots: &[text_slot::TextSlotPayload],
    probe: text_slot::TextSlotPayload,
) -> bool {
    let mut found: bool = false;
    for s in slots.iter() {
        if s.slot == probe.slot && s.text.as_str() == probe.text.as_str() {
            found = true;
        }
    }
    return found;
}
