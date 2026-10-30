// SCE-GENERATED — DO NOT EDIT
// source-hash: 3e850e348999a821018727a6eab60f939f8f51a6e8133ca476ace471d64c40a6
// SCE-MAP: outbound_stale.scxml:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `OutboundStale`, a file of the
// module that calls it: a call is qualified by the enum
// (`OutboundStale.outboundStale(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum OutboundStale {
    public static func outboundStale(_ enqueuedAtMs: Int64, _ now: Int64, _ maxAgeMs: Int64) throws -> Bool {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        var stale: Bool = false
        try sceFailure.throwIfFailed()
        do {
            let sceCond2 = maxAgeMs > 0
            try sceFailure.throwIfFailed()
            if sceCond2 {
                stale = Checked.sub(sceFailure, now, enqueuedAtMs) > maxAgeMs
                try sceFailure.throwIfFailed()
            }
        }
        do {
            let sceReturned: Bool = stale
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
