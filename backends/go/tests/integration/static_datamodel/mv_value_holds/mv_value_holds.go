// SCE-GENERATED — DO NOT EDIT
// source-hash: 9ad3991e659415a87e89be4422403fddaea9105e2a522b988eeacca0944ca255
// SCE-MAP: mv_value_holds.scxml:13 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `mv_value_holds`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package mv_value_holds

import (
	sce_mv_value "github.com/newmassrael/sce-go-tests/integration/static_datamodel/mv_value"
)

func MvValueHolds(values []sce_mv_value.MvValuePayload, probe sce_mv_value.MvValuePayload) bool {
    var found bool = false
    for _, v := range values {
        if (v.Id == probe.Id && v.Field == probe.Field && v.WallTime == probe.WallTime && v.Counter == probe.Counter && v.NodeId == probe.NodeId) {
            found = true;
        }
    }
    return found
}
