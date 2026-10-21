// SCE-GENERATED — DO NOT EDIT
// source-hash: 048fe3b13b4ba344e3f60c0d3a93bf8f8568ab7bd254b4d4763d7e631895e2f6
// SCE-MAP: event_conflict.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `event_conflict`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package event_conflict

import (
	sce_mv_value "github.com/newmassrael/sce-go-tests/integration/static_datamodel/mv_value"
)

func EventConflict(values []sce_mv_value.MvValuePayload, id uint64) bool {
    var twice bool = false
    for _, a := range values {
        if (a.Id == id && (a.Field == 9 || a.Field >= 15 && a.Field <= 58)) {
            for _, b := range values {
                if (b.Id == id && b.Field == a.Field && (b.WallTime != a.WallTime || b.Counter != a.Counter || b.NodeId != a.NodeId)) {
                    twice = true;
                }
            }
        }
    }
    return twice
}
