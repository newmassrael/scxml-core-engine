// SCE-GENERATED — DO NOT EDIT
// source-hash: 9b4cfcaed7ada488617b52a841abc3e598466613b60c7e1131e1820f7a244b47
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
    public static func orderingGapEnd(_ pending: [OrderingSlotPayload], _ next: UInt64, _ now: Int64, _ timeoutMs: Int64) throws -> UInt64 {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        var resume: UInt64 = next
        try sceFailure.throwIfFailed()
        var first: Bool = true
        try sceFailure.throwIfFailed()
        for slot in pending {
            do {
                let sceCond3 = first && slot.seq != next && Checked.sub(sceFailure, now, slot.arrivedAtMs) >= timeoutMs
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    resume = slot.seq
                    try sceFailure.throwIfFailed()
                }
            }
            first = false
            try sceFailure.throwIfFailed()
        }
        do {
            let sceReturned: UInt64 = resume
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
