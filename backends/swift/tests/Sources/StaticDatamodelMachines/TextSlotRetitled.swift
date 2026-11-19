// SCE-GENERATED — DO NOT EDIT
// source-hash: 8efa6e5bdb979d7a72d99505a1b8e8df1cfdca1e9dcbc9650c45b201c9ec78ec
// source: sce:std/text/text_slot_retitled.scxml
// SCE-MAP: text_slot_retitled.scxml:16 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `TextSlotRetitled`, a file of the
// module that calls it: a call is qualified by the enum
// (`TextSlotRetitled.textSlotRetitled(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum TextSlotRetitled {
    public static func textSlotRetitled(_ slots: [TextSlotPayload]) throws(AlgorithmError) -> [TextSlotPayload] {
        var out: [TextSlotPayload] = []
        out.reserveCapacity(16)
        for s in slots {
            var shown: TextSlotPayload = TextSlotPayload(slot: s.slot, version: s.version, text: s.text)
            if s.text.utf8.elementsEqual("".utf8) {
                shown.text = "untitled"
            }
            out.append(shown)
        }
        return out
    }
}
