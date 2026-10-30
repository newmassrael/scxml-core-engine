// SCE-GENERATED — DO NOT EDIT
// source-hash: 1ba579cdc10cbe049a3766817dcc6397d58e8716381e63be0e21ee74788ac719
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
    public static func algorithmHlcTick(_ prev: EventSchemaHlcPayload, _ now: Int64, _ node: UInt64) throws -> EventSchemaHlcPayload {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        var next: EventSchemaHlcPayload = EventSchemaHlcPayload(wallTime: now > prev.wallTime ? now : prev.wallTime, counter: 0, nodeId: node)
        try sceFailure.throwIfFailed()
        do {
            let sceCond2 = next.wallTime == prev.wallTime
            try sceFailure.throwIfFailed()
            if sceCond2 {
                next.counter = Checked.add(sceFailure, prev.counter, 1)
                try sceFailure.throwIfFailed()
            }
        }
        do {
            let sceReturned: EventSchemaHlcPayload = next
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
