// SCE-GENERATED — DO NOT EDIT
// source-hash: f2f8682669670c7aaedbccb3102dd71d7b187eac29a32d3d20509a5bc070c12e
// SCE-MAP: causal_cut.scxml:22 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `CausalCut`, a file of the
// module that calls it: a call is qualified by the enum
// (`CausalCut.causalCut(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum CausalCut {
    public static func causalCut(_ members: UInt32, _ known: [CausalKnownPayload]) -> [CausalEntryPayload] {
        var holders: [EnvelopeIdPayload] = []
        holders.reserveCapacity(64)
        var counted: Bool = false
        for mark in known {
            counted = false
            for holder in holders {
                if holder.hi == mark.holderHi && holder.lo == mark.holderLo {
                    counted = true
                }
            }
            if !counted {
                let member: EnvelopeIdPayload = EnvelopeIdPayload(hi: mark.holderHi, lo: mark.holderLo)
                holders.append(member)
            }
        }
        var out: [CausalEntryPayload] = []
        out.reserveCapacity(64)
        if UInt32(truncatingIfNeeded: holders.count) >= members {
            var done: Bool = false
            var low: UInt64 = 0
            var value: UInt64 = 0
            for author in known {
                done = false
                for cut in out {
                    if cut.hi == author.hi && cut.lo == author.lo {
                        done = true
                    }
                }
                if !done {
                    low = author.seq
                    for peer in holders {
                        value = 0
                        for entry in known {
                            if entry.holderHi == peer.hi && entry.holderLo == peer.lo && entry.hi == author.hi && entry.lo == author.lo {
                                value = entry.seq
                            }
                        }
                        if value < low {
                            low = value
                        }
                    }
                    if low > 0 {
                        let stable: CausalEntryPayload = CausalEntryPayload(hi: author.hi, lo: author.lo, seq: low)
                        out.append(stable)
                    }
                }
            }
        }
        return out
    }
}
