// SCE-GENERATED — DO NOT EDIT
// source-hash: a45b1edeebf5addf07d55990d33be593d588441249c6330a80971f056631ac90
// SCE-MAP: hlc_receive.scxml:19 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `hlc_receive`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package hlc_receive

import (
	scealgorithm "github.com/newmassrael/sce-forge-runtime/algorithm"
	sce_hlc_timestamp "github.com/newmassrael/sce-go-tests/integration/static_datamodel/hlc_timestamp"
)

func HlcReceive(prev sce_hlc_timestamp.HlcTimestampPayload, msg sce_hlc_timestamp.HlcTimestampPayload, now int64, node uint64) (sceValue sce_hlc_timestamp.HlcTimestampPayload, sceErr error) {
    // SCE_FORGE.md §3.4.1: each checked operation records a failure here, and
    // the statement around it returns it before the next statement runs.
    var sceFailure scealgorithm.Failure
    var seen int64 = func() int64 { if prev.WallTime > msg.WallTime { return prev.WallTime }; return msg.WallTime }()
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    next := sce_hlc_timestamp.HlcTimestampPayload{WallTime: func() int64 { if now > seen { return now }; return seen }(), Counter: 0, NodeId: node}
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    if sceCond1 := next.WallTime == prev.WallTime && next.WallTime == msg.WallTime; sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        next.Counter = scealgorithm.AddUint32(&sceFailure, func() uint32 { if prev.Counter > msg.Counter { return prev.Counter }; return msg.Counter }(), 1);
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
    } else {
        if sceCond2 := next.WallTime == prev.WallTime; sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        } else if sceCond2 {
            next.Counter = scealgorithm.AddUint32(&sceFailure, prev.Counter, 1);
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
        } else {
            if sceCond3 := next.WallTime == msg.WallTime; sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            } else if sceCond3 {
                next.Counter = scealgorithm.AddUint32(&sceFailure, msg.Counter, 1);
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
            }
        }
    }
    {
        var sceReturned sce_hlc_timestamp.HlcTimestampPayload = next
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        return sceReturned, nil
    }
}
