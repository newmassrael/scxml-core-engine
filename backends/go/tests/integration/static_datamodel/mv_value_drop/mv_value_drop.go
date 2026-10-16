// SCE-GENERATED — DO NOT EDIT
// source-hash: b8448cd407b1027864e71ab2f9c10d7e6a28146f7cc75313b45ffc4e7287b0f4
// SCE-MAP: mv_value_drop.scxml:16 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `mv_value_drop`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package mv_value_drop

import (
	scealgorithm "github.com/newmassrael/sce-forge-runtime/algorithm"
	sce_mv_value "github.com/newmassrael/sce-go-tests/integration/static_datamodel/mv_value"
)

func MvValueDrop(values []sce_mv_value.MvValuePayload, gone sce_mv_value.MvValuePayload) (sceValue []sce_mv_value.MvValuePayload, sceErr error) {
    // SCE_FORGE.md §3.4.1: each checked operation records a failure here, and
    // the statement around it returns it before the next statement runs.
    var sceFailure scealgorithm.Failure
    out := make([]sce_mv_value.MvValuePayload, 0, 512)
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    for _, v := range values {
        if sceCond2 := v.Id != gone.Id || v.Field != gone.Field || v.WallTime != gone.WallTime || v.Counter != gone.Counter || v.NodeId != gone.NodeId; sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        } else if sceCond2 {
            out = append(out, v)
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
        }
    }
    {
        var sceReturned []sce_mv_value.MvValuePayload = out
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        return sceReturned, nil
    }
}
