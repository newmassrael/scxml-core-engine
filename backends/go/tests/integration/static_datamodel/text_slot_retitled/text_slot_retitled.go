// SCE-GENERATED — DO NOT EDIT
// source-hash: 848898b5bbb016bf799fdf5a1235f8e512e7516e53f20e55bfd5887716ace728
// SCE-MAP: text_slot_retitled.scxml:16 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `text_slot_retitled`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package text_slot_retitled

import (
	scealgorithm "github.com/newmassrael/sce-forge-runtime/algorithm"
	sce_text_slot "github.com/newmassrael/sce-go-tests/integration/static_datamodel/text_slot"
)

func TextSlotRetitled(slots []sce_text_slot.TextSlotPayload) (sceValue []sce_text_slot.TextSlotPayload, sceErr error) {
    // SCE_FORGE.md §3.4.1: each checked operation records a failure here, and
    // the statement around it returns it before the next statement runs.
    var sceFailure scealgorithm.Failure
    out := make([]sce_text_slot.TextSlotPayload, 0, 16)
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    for _, s := range slots {
        shown := sce_text_slot.TextSlotPayload{Slot: s.Slot, Version: s.Version, Text: s.Text}
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        if sceCond2 := s.Text == ""; sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        } else if sceCond2 {
            shown.Text = "untitled";
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
        }
        out = append(out, shown)
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
    }
    {
        var sceReturned []sce_text_slot.TextSlotPayload = out
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        return sceReturned, nil
    }
}
