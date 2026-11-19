// SCE-GENERATED — DO NOT EDIT
// source-hash: 55b8b6461c4ad85115fa2e7767977c95fb0aeba57c5c4a2ae6286cb9cf4b9bf5
// source: sce:std/mesh/dedup_holds.scxml
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
