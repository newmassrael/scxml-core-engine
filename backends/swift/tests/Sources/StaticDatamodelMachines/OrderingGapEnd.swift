// SCE-GENERATED — DO NOT EDIT
// source-hash: 797e84fd863bfbc739480c7874ce5e86a5db5170c14af387baa105b115d72480
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
