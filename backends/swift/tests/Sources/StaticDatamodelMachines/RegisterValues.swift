// SCE-GENERATED — DO NOT EDIT
// source-hash: 81cc59c6b8f46736a80b4b69ee489274d9b0ae32efae821805ba44c3bac3af2c
// SCE-MAP: register_values.scxml:21 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `RegisterValues`, a file of the
// module that calls it: a call is qualified by the enum
// (`RegisterValues.registerValues(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum RegisterValues {
    public static func registerValues(_ values: [MvValuePayload], _ id: UInt64, _ field: UInt64) throws(AlgorithmError) -> [MvValuePayload] {
        var out: [MvValuePayload] = []
        out.reserveCapacity(512)
        var started: Bool = false
        var lastWall: Int64 = 0
        var lastCounter: UInt32 = 0
        var lastNode: UInt64 = 0
        var more: Bool = true
        while more {
            var chosen: Bool = false
            var bestWall: Int64 = 0
            var bestCounter: UInt32 = 0
            var bestNode: UInt64 = 0
            for v in values {
                if v.id == id && v.field == field && (!started || v.wallTime > lastWall || v.wallTime == lastWall && (v.counter > lastCounter || v.counter == lastCounter && v.nodeId > lastNode)) && (!chosen || v.wallTime < bestWall || v.wallTime == bestWall && (v.counter < bestCounter || v.counter == bestCounter && v.nodeId < bestNode)) {
                    chosen = true
                    bestWall = v.wallTime
                    bestCounter = v.counter
                    bestNode = v.nodeId
                }
            }
            if chosen {
                for w in values {
                    if w.id == id && w.field == field && w.wallTime == bestWall && w.counter == bestCounter && w.nodeId == bestNode {
                        out.append(w)
                    }
                }
                started = true
                lastWall = bestWall
                lastCounter = bestCounter
                lastNode = bestNode
            } else {
                more = false
            }
        }
        return out
    }
}
