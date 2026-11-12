// SCE-GENERATED — DO NOT EDIT
// source-hash: fb4f42a34e7ac40a28d8e5daee5d633c3c45dddac513816edf2bb83fdc85f91c
// SCE-MAP: mv_value_drop.scxml:16 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `MvValueDrop`, a file of the
// module that calls it: a call is qualified by the enum
// (`MvValueDrop.mvValueDrop(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum MvValueDrop {
    public static func mvValueDrop(_ values: [MvValuePayload], _ gone: MvValuePayload) throws(AlgorithmError) -> [MvValuePayload] {
        var out: [MvValuePayload] = []
        out.reserveCapacity(512)
        for v in values {
            if v.id != gone.id || v.field != gone.field || v.wallTime != gone.wallTime || v.counter != gone.counter || v.nodeId != gone.nodeId {
                out.append(v)
            }
        }
        return out
    }
}
