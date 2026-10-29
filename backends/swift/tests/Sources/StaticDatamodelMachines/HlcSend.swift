// SCE-GENERATED — DO NOT EDIT
// source-hash: b3df842d016676bf8989832cb164de51027fadf6ca9e7b5543c7bac0c9a13d1a
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
