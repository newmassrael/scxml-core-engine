// SCE-GENERATED — DO NOT EDIT
// source-hash: 3d8e77ae6e1148d1818998f2f9d129b014c767dc3cded9cb0af2d4fc0c10af0a
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
    public static func outboundStale(_ enqueuedAtMs: Int64, _ now: Int64, _ maxAgeMs: Int64) throws(AlgorithmError) -> Bool {
        var stale: Bool = false
        if maxAgeMs > 0 {
            stale = (try Checked.sub(now, enqueuedAtMs)) > maxAgeMs
        }
        return stale
    }
}
