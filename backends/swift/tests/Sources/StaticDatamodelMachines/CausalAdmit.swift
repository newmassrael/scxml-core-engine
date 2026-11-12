// SCE-GENERATED — DO NOT EDIT
// source-hash: 3d8e77ae6e1148d1818998f2f9d129b014c767dc3cded9cb0af2d4fc0c10af0a
// SCE-MAP: causal_admit.scxml:25 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `CausalAdmit`, a file of the
// module that calls it: a call is qualified by the enum
// (`CausalAdmit.causalAdmit(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum CausalAdmit {
    public static func causalAdmit(_ delivered: [CausalEntryPayload], _ dot: CausalEntryPayload, _ context: [CausalEntryPayload]) throws(AlgorithmError) -> CausalAdmission {
        var seen: UInt64 = 0
        for mine in delivered {
            if mine.hi == dot.hi && mine.lo == dot.lo {
                seen = mine.seq
            }
        }
        var covered: Bool = true
        var have: UInt64 = 0
        for needed in context {
            have = 0
            for held in delivered {
                if held.hi == needed.hi && held.lo == needed.lo {
                    have = held.seq
                }
            }
            if (needed.hi != dot.hi || needed.lo != dot.lo) && needed.seq > have {
                covered = false
            }
        }
        var result: CausalAdmission = CausalAdmission.hold
        if dot.seq <= seen {
            result = CausalAdmission.drop
        }
        if dot.seq == (try Checked.add(seen, 1)) && covered {
            result = CausalAdmission.deliver
        }
        return result
    }
}
