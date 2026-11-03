// SCE-GENERATED — DO NOT EDIT
// source-hash: 848898b5bbb016bf799fdf5a1235f8e512e7516e53f20e55bfd5887716ace728
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
