// SCE-GENERATED — DO NOT EDIT
// source-hash: fb4f42a34e7ac40a28d8e5daee5d633c3c45dddac513816edf2bb83fdc85f91c
// SCE-MAP: hlc_send.scxml:17 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `HlcSend`, a file of the
// module that calls it: a call is qualified by the enum
// (`HlcSend.hlcSend(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum HlcSend {
    public static func hlcSend(_ prev: HlcTimestampPayload, _ now: Int64, _ node: UInt64) throws -> HlcTimestampPayload {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        var next: HlcTimestampPayload = HlcTimestampPayload(wallTime: now > prev.wallTime ? now : prev.wallTime, counter: 0, nodeId: node)
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
            let sceReturned: HlcTimestampPayload = next
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
