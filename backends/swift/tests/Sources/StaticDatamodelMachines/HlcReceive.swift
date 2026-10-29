// SCE-GENERATED — DO NOT EDIT
// source-hash: b3df842d016676bf8989832cb164de51027fadf6ca9e7b5543c7bac0c9a13d1a
// SCE-MAP: hlc_receive.scxml:19 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `HlcReceive`, a file of the
// module that calls it: a call is qualified by the enum
// (`HlcReceive.hlcReceive(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum HlcReceive {
    public static func hlcReceive(_ prev: HlcTimestampPayload, _ msg: HlcTimestampPayload, _ now: Int64, _ node: UInt64) throws -> HlcTimestampPayload {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        let seen: Int64 = prev.wallTime > msg.wallTime ? prev.wallTime : msg.wallTime
        try sceFailure.throwIfFailed()
        var next: HlcTimestampPayload = HlcTimestampPayload(wallTime: now > seen ? now : seen, counter: 0, nodeId: node)
        try sceFailure.throwIfFailed()
        do {
            let sceCond2 = next.wallTime == prev.wallTime && next.wallTime == msg.wallTime
            try sceFailure.throwIfFailed()
            if sceCond2 {
                next.counter = Checked.add(sceFailure, prev.counter > msg.counter ? prev.counter : msg.counter, 1)
                try sceFailure.throwIfFailed()
            } else {
                do {
                    let sceCond4 = next.wallTime == prev.wallTime
                    try sceFailure.throwIfFailed()
                    if sceCond4 {
                        next.counter = Checked.add(sceFailure, prev.counter, 1)
                        try sceFailure.throwIfFailed()
                    } else {
                        do {
                            let sceCond6 = next.wallTime == msg.wallTime
                            try sceFailure.throwIfFailed()
                            if sceCond6 {
                                next.counter = Checked.add(sceFailure, msg.counter, 1)
                                try sceFailure.throwIfFailed()
                            }
                        }
                    }
                }
            }
        }
        do {
            let sceReturned: HlcTimestampPayload = next
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
