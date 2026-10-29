// SCE-GENERATED — DO NOT EDIT
// source-hash: 9b33efdfdb5f3c283f92ca6bd6336ef9097b5ad8d1795089f76b27911b6fde36
// SCE-MAP: gcra_admit.scxml:35 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `GcraAdmit`, a file of the
// module that calls it: a call is qualified by the enum
// (`GcraAdmit.gcraAdmit(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum GcraAdmit {
    public static func gcraAdmit(_ tat: Int64, _ now: Int64, _ cost: UInt32, _ interval: Int64, _ burst: UInt32) throws -> GcraDecisionPayload {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        do {
            let sceCond2 = !(cost >= 1 && interval >= 1 && burst >= 1 && cost <= burst)
            try sceFailure.throwIfFailed()
            if sceCond2 {
                throw AlgorithmError.precondition
            }
        }
        let start: Int64 = tat > now ? tat : now
        try sceFailure.throwIfFailed()
        let arrival: Int64 = Checked.add(sceFailure, start, Checked.mul(sceFailure, Int64(cost), interval))
        try sceFailure.throwIfFailed()
        let limit: Int64 = Checked.add(sceFailure, now, Checked.mul(sceFailure, Int64(burst), interval))
        try sceFailure.throwIfFailed()
        let decision: GcraDecisionPayload = GcraDecisionPayload(admitted: arrival <= limit, tat: arrival <= limit ? arrival : tat, retryAfter: arrival <= limit ? 0 : Checked.sub(sceFailure, arrival, limit))
        try sceFailure.throwIfFailed()
        do {
            let sceReturned: GcraDecisionPayload = decision
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
