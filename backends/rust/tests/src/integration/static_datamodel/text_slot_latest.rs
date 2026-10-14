// SCE-GENERATED — DO NOT EDIT
// source-hash: f7347a8b98455a65e31ae2b3af12ef65d68a284242bd71a8102ab373827910fd
#![doc = "SCE-MAP: text_slot_latest.scxml:18 :: _forge_body"]
// SCE-MAP: text_slot_latest.scxml:18 :: _forge_body

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
pub fn text_slot_latest(
    a: text_slot::TextSlotPayload,
    b: text_slot::TextSlotPayload,
) -> text_slot::TextSlotPayload {
    let mut out = text_slot::TextSlotPayload {
        slot: a.slot,
        version: a.version,
        text: ::sce_portable_bytes::SceString::<32>::widened(&a.text),
    };
    if b.version > a.version {
        out.version = b.version;
        if b.text.as_str() != a.text.as_str() {
            out.text = ::sce_portable_bytes::SceString::<32>::widened(&b.text);
        }
    }
    return out;
}
