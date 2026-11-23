// SCE-GENERATED — DO NOT EDIT
// source-hash: bb79893ae8ff05aa98d88701952b9689dfd413403f164f7a9bab8d73d125746f
// source: algorithm_slot_titled.scxml
// SCE-MAP: algorithm_slot_titled.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `slot_titled`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package slot_titled

import (
	sce_text_slot "github.com/newmassrael/sce-go-tests/integration/static_datamodel/text_slot"
)

func SlotTitled(title string, slots []sce_text_slot.TextSlotPayload) bool {
    var found bool = false
    for _, s := range slots {
        if (s.Text == title) {
            found = true;
        }
    }
    return found
}
