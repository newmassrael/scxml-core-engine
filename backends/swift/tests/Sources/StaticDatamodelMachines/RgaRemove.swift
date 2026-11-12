// SCE-GENERATED — DO NOT EDIT
// source-hash: fb4f42a34e7ac40a28d8e5daee5d633c3c45dddac513816edf2bb83fdc85f91c
// SCE-MAP: rga_remove.scxml:20 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `RgaRemove`, a file of the
// module that calls it: a call is qualified by the enum
// (`RgaRemove.rgaRemove(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum RgaRemove {
    public static func rgaRemove(_ gone: [RgaGonePayload], _ removal: RgaGonePayload) throws(AlgorithmError) -> [RgaGonePayload] {
        var out: [RgaGonePayload] = []
        out.reserveCapacity(256)
        var held: Bool = false
        for g in gone {
            out.append(g)
            if g.key == removal.key {
                held = true
            }
        }
        if !held {
            out.append(removal)
        }
        return out
    }
}
