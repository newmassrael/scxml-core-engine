// SCE-GENERATED — DO NOT EDIT
// source-hash: 9b4cfcaed7ada488617b52a841abc3e598466613b60c7e1131e1820f7a244b47
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
    public static func orderingDrain(_ pending: [OrderingSlotPayload], _ start: UInt64) throws -> UInt64 {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        var next: UInt64 = start
        try sceFailure.throwIfFailed()
        for slot in pending {
            do {
                let sceCond3 = slot.seq == next
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    next = Checked.add(sceFailure, next, 1)
                    try sceFailure.throwIfFailed()
                }
            }
        }
        do {
            let sceReturned: UInt64 = next
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
