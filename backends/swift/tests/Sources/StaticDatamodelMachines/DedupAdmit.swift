// SCE-GENERATED — DO NOT EDIT
// source-hash: 3d8e77ae6e1148d1818998f2f9d129b014c767dc3cded9cb0af2d4fc0c10af0a
// SCE-MAP: dedup_admit.scxml:26 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `DedupAdmit`, a file of the
// module that calls it: a call is qualified by the enum
// (`DedupAdmit.dedupAdmit(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum DedupAdmit {
    public static func dedupAdmit(_ window: [EnvelopeIdPayload], _ id: EnvelopeIdPayload, _ capacity: UInt32) throws(AlgorithmError) -> [EnvelopeIdPayload] {
        if !(capacity > 0 && UInt32(truncatingIfNeeded: window.count) <= capacity) {
            throw AlgorithmError.precondition
        }
        var held: Bool = false
        for x in window {
            if x.hi == id.hi && x.lo == id.lo {
                held = true
            }
        }
        var out: [EnvelopeIdPayload] = []
        out.reserveCapacity(256)
        let skip: UInt32 = held || UInt32(truncatingIfNeeded: window.count) < capacity ? 0 : 1
        var i: UInt32 = 0
        for kept in window {
            if i >= skip {
                out.append(kept)
            }
            i = (try Checked.add(i, 1))
        }
        if !held {
            out.append(id)
        }
        return out
    }
}
