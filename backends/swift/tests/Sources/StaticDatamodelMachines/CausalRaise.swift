// SCE-GENERATED — DO NOT EDIT
// source-hash: 3d8e77ae6e1148d1818998f2f9d129b014c767dc3cded9cb0af2d4fc0c10af0a
// SCE-MAP: causal_raise.scxml:21 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `CausalRaise`, a file of the
// module that calls it: a call is qualified by the enum
// (`CausalRaise.causalRaise(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum CausalRaise {
    public static func causalRaise(_ vector: [CausalEntryPayload], _ other: [CausalEntryPayload]) -> [CausalEntryPayload] {
        var out: [CausalEntryPayload] = []
        out.reserveCapacity(64)
        var top: UInt64 = 0
        for mine in vector {
            top = mine.seq
            for theirs in other {
                if theirs.hi == mine.hi && theirs.lo == mine.lo && theirs.seq > top {
                    top = theirs.seq
                }
            }
            let raised: CausalEntryPayload = CausalEntryPayload(hi: mine.hi, lo: mine.lo, seq: top)
            out.append(raised)
        }
        var named: Bool = false
        for added in other {
            named = false
            for known in vector {
                if known.hi == added.hi && known.lo == added.lo {
                    named = true
                }
            }
            if !named {
                out.append(added)
            }
        }
        return out
    }
}
