// SCE-GENERATED — DO NOT EDIT
// source-hash: 43faded87b84ec06f387d0e3d6ccb626e2fa1802d2395c65b70588a4c3c394af
// SCE-MAP: event_latest.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `event_latest`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package event_latest

import (
	sce_mv_value "github.com/newmassrael/sce-go-tests/integration/static_datamodel/mv_value"
	sce_event_later "github.com/newmassrael/sce-go-tests/integration/static_datamodel/event_later"
)

func EventLatest(values []sce_mv_value.MvValuePayload, id uint64, field uint64, none int64) int64 {
    var found bool = false
    var bestWall int64 = 0
    var bestCounter uint32 = 0
    var bestNode uint64 = 0
    var best int64 = none
    for _, v := range values {
        if (v.Id == id && v.Field == field && (!found || sce_event_later.EventLater(v.WallTime, v.Counter, v.NodeId, bestWall, bestCounter, bestNode))) {
            found = true;
            bestWall = v.WallTime;
            bestCounter = v.Counter;
            bestNode = v.NodeId;
            best = v.Value;
        }
    }
    return best
}
