// SCE-GENERATED — DO NOT EDIT
// source-hash: 1ee4c478fcd6af0bb675edd6c8e90348072aad4cd4e5fb9446a5e52706545b90
// SCE-MAP: algorithm_hlc_tick.scxml:16 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `AlgorithmHlcTick`, a file of the
// module that calls it: a call is qualified by the enum
// (`AlgorithmHlcTick.algorithmHlcTick(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum AlgorithmHlcTick {
    public static func algorithmHlcTick(_ prev: EventSchemaHlcPayload, _ now: Int64, _ node: UInt64) throws(AlgorithmError) -> EventSchemaHlcPayload {
        var next: EventSchemaHlcPayload = EventSchemaHlcPayload(wallTime: now > prev.wallTime ? now : prev.wallTime, counter: 0, nodeId: node)
        if next.wallTime == prev.wallTime {
            next.counter = (try Checked.add(prev.counter, 1))
        }
        return next
    }
}
