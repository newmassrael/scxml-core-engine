// SCE-GENERATED — DO NOT EDIT
// source-hash: 848898b5bbb016bf799fdf5a1235f8e512e7516e53f20e55bfd5887716ace728
#![doc = "SCE-MAP: text_slot_retitled.scxml:16 :: _forge_body"]
// SCE-MAP: text_slot_retitled.scxml:16 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

use super::text_slot;

use sce_portable_bytes::SceOwnedList;

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn text_slot_retitled(
    slots: &[text_slot::TextSlotPayload],
    out: &mut SceOwnedList<text_slot::TextSlotPayload, 16>,
) -> Result<(), sce_forge_runtime::algorithm::AlgorithmError> {
    out.clear();
    out.reserve(16);
    for s in slots.iter() {
        let mut shown = text_slot::TextSlotPayload {
            slot: s.slot,
            version: s.version,
            text: ::sce_portable_bytes::SceString::<32>::widened(&s.text),
        };
        if s.text.as_str() == "" {
            shown.text = ::sce_portable_bytes::sce_text!(32, "untitled");
        }
        out.push(shown.clone())?;
    }
    return Ok(());
}
