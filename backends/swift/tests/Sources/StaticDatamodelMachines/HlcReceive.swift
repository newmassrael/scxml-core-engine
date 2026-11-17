// SCE-GENERATED — DO NOT EDIT
// source-hash: 451ed84f90f73b42f48909eca6f7a302d6cb0b852dd68c846f5e713406957876
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
    public static func hlcReceive(_ prev: HlcTimestampPayload, _ msg: HlcTimestampPayload, _ now: Int64, _ node: UInt64) throws(AlgorithmError) -> HlcTimestampPayload {
        let seen: Int64 = prev.wallTime > msg.wallTime ? prev.wallTime : msg.wallTime
        var next: HlcTimestampPayload = HlcTimestampPayload(wallTime: now > seen ? now : seen, counter: 0, nodeId: node)
        if next.wallTime == prev.wallTime && next.wallTime == msg.wallTime {
            next.counter = (try Checked.add(prev.counter > msg.counter ? prev.counter : msg.counter, 1))
        } else {
            if next.wallTime == prev.wallTime {
                next.counter = (try Checked.add(prev.counter, 1))
            } else {
                if next.wallTime == msg.wallTime {
                    next.counter = (try Checked.add(msg.counter, 1))
                }
            }
        }
        return next
    }
}
