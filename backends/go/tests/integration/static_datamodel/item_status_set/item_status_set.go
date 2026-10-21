// SCE-GENERATED — DO NOT EDIT
// source-hash: 602cf8b2e076ca5de83736526fa544c7671ce4b8406bed6ee323cbe2df13499b
// SCE-MAP: algorithm_item_status_set.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `item_status_set`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package item_status_set

import (
	scealgorithm "github.com/newmassrael/sce-forge-runtime/algorithm"
	sce_schema_item "github.com/newmassrael/sce-go-tests/integration/static_datamodel/schema_item"
	sce_enum_item_status "github.com/newmassrael/sce-go-tests/integration/static_datamodel/enum_item_status"
)

func ItemStatusSet(items []sce_schema_item.SchemaItemPayload, id uint32, status sce_enum_item_status.EnumItemStatus) (sceValue []sce_schema_item.SchemaItemPayload, sceErr error) {
    // SCE_FORGE.md §3.4.1: each checked operation records a failure here, and
    // the statement around it returns it before the next statement runs.
    var sceFailure scealgorithm.Failure
    moved := sce_schema_item.SchemaItemPayload{Id: id, Status: status}
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    out := make([]sce_schema_item.SchemaItemPayload, 0, 4)
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    for _, e := range items {
        if sceCond2 := e.Id == id; sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        } else if sceCond2 {
            out = append(out, moved)
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
        } else {
            out = append(out, e)
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
        }
    }
    {
        var sceReturned []sce_schema_item.SchemaItemPayload = out
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        return sceReturned, nil
    }
}
