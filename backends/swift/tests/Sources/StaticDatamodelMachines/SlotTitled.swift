// SCE-GENERATED — DO NOT EDIT
// source-hash: bb79893ae8ff05aa98d88701952b9689dfd413403f164f7a9bab8d73d125746f
// source: algorithm_slot_titled.scxml
// SCE-MAP: algorithm_slot_titled.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `SlotTitled`, a file of the
// module that calls it: a call is qualified by the enum
// (`SlotTitled.slotTitled(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum SlotTitled {
    public static func slotTitled(_ title: String, _ slots: [TextSlotPayload]) -> Bool {
        var found: Bool = false
        for s in slots {
            if s.text.utf8.elementsEqual(title.utf8) {
                found = true
            }
        }
        return found
    }
}
