// SCE-GENERATED — DO NOT EDIT
// source-hash: 438f3c44a0f49109d71c51cacca59b20d771e2147409672affa707c8d139f3c1
// SCE-MAP: algorithm_ledger_bump.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `ledger_bump`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package ledger_bump

import (
	scealgorithm "github.com/newmassrael/sce-forge-runtime/algorithm"
	sce_schema_total "github.com/newmassrael/sce-go-tests/integration/static_datamodel/schema_total"
)

func LedgerBump(total sce_schema_total.SchemaTotalPayload, delta int32) (sceValue sce_schema_total.SchemaTotalPayload, sceErr error) {
    // SCE_FORGE.md §3.4.1: each checked operation records a failure here, and
    // the statement around it returns it before the next statement runs.
    var sceFailure scealgorithm.Failure
    next := sce_schema_total.SchemaTotalPayload{Count: scealgorithm.AddUint32(&sceFailure, total.Count, 1), Sum: scealgorithm.AddInt32(&sceFailure, total.Sum, delta)}
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    {
        var sceReturned sce_schema_total.SchemaTotalPayload = next
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        return sceReturned, nil
    }
}
