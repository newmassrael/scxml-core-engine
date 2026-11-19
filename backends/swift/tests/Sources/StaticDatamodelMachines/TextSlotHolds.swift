// SCE-GENERATED — DO NOT EDIT
// source-hash: 62e099c55c3af43d1033820a3493a3d8ff5082ff405738e08864fc63e2adbb84
// source: sce:std/text/text_slot_holds.scxml
// SCE-MAP: text_slot_holds.scxml:14 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `TextSlotHolds`, a file of the
// module that calls it: a call is qualified by the enum
// (`TextSlotHolds.textSlotHolds(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum TextSlotHolds {
    public static func textSlotHolds(_ slots: [TextSlotPayload], _ probe: TextSlotPayload) -> Bool {
        var found: Bool = false
        for s in slots {
            if s.slot == probe.slot && s.text.utf8.elementsEqual(probe.text.utf8) {
                found = true
            }
        }
        return found
    }
}
