// SCE-GENERATED — DO NOT EDIT
// source-hash: adbae20e9ca5d62b09870f4a9e5e88119b90dbcf69963958bb021c42c5c2c6a4
// SCE-MAP: algorithm_item_status_count.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `item_status_count`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package item_status_count

import (
	scealgorithm "github.com/newmassrael/sce-forge-runtime/algorithm"
	sce_schema_item "github.com/newmassrael/sce-go-tests/integration/static_datamodel/schema_item"
	sce_enum_item_status "github.com/newmassrael/sce-go-tests/integration/static_datamodel/enum_item_status"
)

func ItemStatusCount(items []sce_schema_item.SchemaItemPayload, status sce_enum_item_status.EnumItemStatus) (sceValue uint32, sceErr error) {
    // SCE_FORGE.md §3.4.1: each checked operation records a failure here, and
    // the statement around it returns it before the next statement runs.
    var sceFailure scealgorithm.Failure
    var count uint32 = 0
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    for _, e := range items {
        if sceCond2 := e.Status == status; sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        } else if sceCond2 {
            count = scealgorithm.AddUint32(&sceFailure, count, 1);
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
        }
    }
    {
        var sceReturned uint32 = count
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        return sceReturned, nil
    }
}
