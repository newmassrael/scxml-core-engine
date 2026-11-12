// SCE-GENERATED — DO NOT EDIT
// source-hash: fb4f42a34e7ac40a28d8e5daee5d633c3c45dddac513816edf2bb83fdc85f91c
// SCE-MAP: mv_value_keep.scxml:22 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `MvValueKeep`, a file of the
// module that calls it: a call is qualified by the enum
// (`MvValueKeep.mvValueKeep(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum MvValueKeep {
    public static func mvValueKeep(_ values: [MvValuePayload], _ held: MvValuePayload) throws(AlgorithmError) -> [MvValuePayload] {
        var out: [MvValuePayload] = []
        out.reserveCapacity(512)
        var there: Bool = false
        for v in values {
            if v.id == held.id && v.field == held.field && v.wallTime == held.wallTime && v.counter == held.counter && v.nodeId == held.nodeId {
                there = true
            }
            out.append(v)
        }
        if !there {
            out.append(held)
        }
        return out
    }
}
