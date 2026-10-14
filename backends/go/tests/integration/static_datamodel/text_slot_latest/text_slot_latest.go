// SCE-GENERATED — DO NOT EDIT
// source-hash: f7347a8b98455a65e31ae2b3af12ef65d68a284242bd71a8102ab373827910fd
// SCE-MAP: text_slot_latest.scxml:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `text_slot_latest`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package text_slot_latest

import (
	sce_text_slot "github.com/newmassrael/sce-go-tests/integration/static_datamodel/text_slot"
)

func TextSlotLatest(a sce_text_slot.TextSlotPayload, b sce_text_slot.TextSlotPayload) sce_text_slot.TextSlotPayload {
    out := sce_text_slot.TextSlotPayload{Slot: a.Slot, Version: a.Version, Text: a.Text}
    if (b.Version > a.Version) {
        out.Version = b.Version;
        if (b.Text != a.Text) {
            out.Text = b.Text;
        }
    }
    return out
}
