// SCE-GENERATED — DO NOT EDIT
// source-hash: 486a7ede17aec50321702f8e51d91329868b60553618b2452672e3f16a3437c4
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
