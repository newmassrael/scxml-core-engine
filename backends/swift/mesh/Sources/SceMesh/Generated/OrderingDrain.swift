// SCE-GENERATED — DO NOT EDIT
// source-hash: 52868da22c06561e2c560050fc709417107e0518328e8ceb0790994e53be08ca
// SCE-MAP: ordering_drain.scxml:15 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `OrderingDrain`, a file of the
// module that calls it: a call is qualified by the enum
// (`OrderingDrain.orderingDrain(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum OrderingDrain {
    public static func orderingDrain(_ pending: [OrderingSlotPayload], _ start: UInt64) throws(AlgorithmError) -> UInt64 {
        var next: UInt64 = start
        for slot in pending {
            if slot.seq == next {
                next = (try Checked.add(next, 1))
            }
        }
        return next
    }
}
