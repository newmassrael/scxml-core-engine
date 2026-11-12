// SCE-GENERATED — DO NOT EDIT
// source-hash: 3d8e77ae6e1148d1818998f2f9d129b014c767dc3cded9cb0af2d4fc0c10af0a
// SCE-MAP: ordering_gap_end.scxml:21 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `OrderingGapEnd`, a file of the
// module that calls it: a call is qualified by the enum
// (`OrderingGapEnd.orderingGapEnd(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum OrderingGapEnd {
    public static func orderingGapEnd(_ pending: [OrderingSlotPayload], _ next: UInt64, _ now: Int64, _ timeoutMs: Int64) throws(AlgorithmError) -> UInt64 {
        var resume: UInt64 = next
        var first: Bool = true
        for slot in pending {
            if (try Checked.and(first && slot.seq != next, { () throws(AlgorithmError) -> Bool in (try Checked.sub(now, slot.arrivedAtMs)) >= timeoutMs })) {
                resume = slot.seq
            }
            first = false
        }
        return resume
    }
}
