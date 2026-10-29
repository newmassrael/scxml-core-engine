// SCE-GENERATED — DO NOT EDIT
// source-hash: b3df842d016676bf8989832cb164de51027fadf6ca9e7b5543c7bac0c9a13d1a
// SCE-MAP: rga_holds.scxml:15 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `RgaHolds`, a file of the
// module that calls it: a call is qualified by the enum
// (`RgaHolds.rgaHolds(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum RgaHolds {
    public static func rgaHolds(_ nodes: [RgaNodePayload], _ key: UInt64) -> Bool {
        var found: Bool = false
        for n in nodes {
            if n.key == key {
                found = true
            }
        }
        return found
    }
}
