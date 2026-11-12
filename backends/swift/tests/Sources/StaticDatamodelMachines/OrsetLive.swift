// SCE-GENERATED — DO NOT EDIT
// source-hash: fb4f42a34e7ac40a28d8e5daee5d633c3c45dddac513816edf2bb83fdc85f91c
// SCE-MAP: orset_live.scxml:31 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `OrsetLive`, a file of the
// module that calls it: a call is qualified by the enum
// (`OrsetLive.orsetLive(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum OrsetLive {
    public static func orsetLive(_ adds: [OrsetEntryPayload], _ tombstones: [OrsetEntryPayload]) throws(AlgorithmError) -> [OrsetEntryPayload] {
        var out: [OrsetEntryPayload] = []
        out.reserveCapacity(1024)
        for x in adds {
            var removed: Bool = false
            for t in tombstones {
                if t.element == x.element && t.wallTime == x.wallTime && t.counter == x.counter && t.nodeId == x.nodeId {
                    removed = true
                }
            }
            if !removed {
                out.append(x)
            }
        }
        return out
    }
}
