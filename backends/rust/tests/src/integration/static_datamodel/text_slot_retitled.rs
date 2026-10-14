// SCE-GENERATED — DO NOT EDIT
// source-hash: f7347a8b98455a65e31ae2b3af12ef65d68a284242bd71a8102ab373827910fd
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
) -> Result<
    SceOwnedList<text_slot::TextSlotPayload, 16>,
    sce_forge_runtime::algorithm::AlgorithmError,
> {
    let mut out: SceOwnedList<text_slot::TextSlotPayload, 16> = SceOwnedList::new();
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
    return Ok(out);
}
