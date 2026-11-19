// SCE-GENERATED — DO NOT EDIT
// source-hash: 828c981ef176c004f9b2a50e715e24fc02170ce678e1d651076f5581f418e25e
// source: sce:std/mesh/ordering_prune.scxml
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
