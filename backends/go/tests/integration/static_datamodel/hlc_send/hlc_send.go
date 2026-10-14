// SCE-GENERATED — DO NOT EDIT
// source-hash: 0a170bbc1ca3025d9cc4009c0838fff8e752b3322334de465b99571c709f4da7
// SCE-MAP: hlc_send.scxml:17 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `hlc_send`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package hlc_send

import (
	scealgorithm "github.com/newmassrael/sce-forge-runtime/algorithm"
	sce_hlc_timestamp "github.com/newmassrael/sce-go-tests/integration/static_datamodel/hlc_timestamp"
)

func HlcSend(prev sce_hlc_timestamp.HlcTimestampPayload, now int64, node uint64) (sceValue sce_hlc_timestamp.HlcTimestampPayload, sceErr error) {
    // SCE_FORGE.md §3.4.1: each checked operation records a failure here, and
    // the statement around it returns it before the next statement runs.
    var sceFailure scealgorithm.Failure
    next := sce_hlc_timestamp.HlcTimestampPayload{WallTime: func() int64 { if now > prev.WallTime { return now }; return prev.WallTime }(), Counter: 0, NodeId: node}
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    if sceCond1 := next.WallTime == prev.WallTime; sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        next.Counter = scealgorithm.AddUint32(&sceFailure, prev.Counter, 1);
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
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
