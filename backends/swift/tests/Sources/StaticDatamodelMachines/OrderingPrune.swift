// SCE-GENERATED — DO NOT EDIT
// source-hash: 3e850e348999a821018727a6eab60f939f8f51a6e8133ca476ace471d64c40a6
// SCE-MAP: ordering_prune.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `OrderingPrune`, a file of the
// module that calls it: a call is qualified by the enum
// (`OrderingPrune.orderingPrune(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum OrderingPrune {
    public static func orderingPrune(_ pending: [OrderingSlotPayload], _ next: UInt64) -> [OrderingSlotPayload] {
        var out: [OrderingSlotPayload] = []
        out.reserveCapacity(256)
        for slot in pending {
            if slot.seq >= next {
                out.append(slot)
            }
        }
        return out
    }
}
