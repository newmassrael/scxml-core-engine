// SCE-GENERATED — DO NOT EDIT
// source-hash: 3d8e77ae6e1148d1818998f2f9d129b014c767dc3cded9cb0af2d4fc0c10af0a
// SCE-MAP: dedup_holds.scxml:15 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `DedupHolds`, a file of the
// module that calls it: a call is qualified by the enum
// (`DedupHolds.dedupHolds(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum DedupHolds {
    public static func dedupHolds(_ window: [EnvelopeIdPayload], _ id: EnvelopeIdPayload) -> Bool {
        var held: Bool = false
        for x in window {
            if x.hi == id.hi && x.lo == id.lo {
                held = true
            }
        }
        return held
    }
}
