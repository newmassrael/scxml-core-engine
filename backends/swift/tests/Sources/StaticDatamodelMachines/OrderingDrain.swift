// SCE-GENERATED — DO NOT EDIT
// source-hash: 3d8e77ae6e1148d1818998f2f9d129b014c767dc3cded9cb0af2d4fc0c10af0a
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
