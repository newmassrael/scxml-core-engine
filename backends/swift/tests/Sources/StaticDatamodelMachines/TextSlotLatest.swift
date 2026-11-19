// SCE-GENERATED — DO NOT EDIT
// source-hash: ffaa190a07e30df25d3080cb0ae2b255b15e12109f4d095a6651dc58c3ed152a
// source: sce:std/text/text_slot_latest.scxml
// SCE-MAP: text_slot_latest.scxml:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `TextSlotLatest`, a file of the
// module that calls it: a call is qualified by the enum
// (`TextSlotLatest.textSlotLatest(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum TextSlotLatest {
    public static func textSlotLatest(_ a: TextSlotPayload, _ b: TextSlotPayload) -> TextSlotPayload {
        var out: TextSlotPayload = TextSlotPayload(slot: a.slot, version: a.version, text: a.text)
        if b.version > a.version {
            out.version = b.version
            if !b.text.utf8.elementsEqual(a.text.utf8) {
                out.text = b.text
            }
        }
        return out
    }
}
