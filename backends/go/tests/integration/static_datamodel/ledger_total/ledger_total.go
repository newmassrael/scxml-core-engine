// SCE-GENERATED — DO NOT EDIT
// source-hash: 813d6364249fab80a1abeae788a84cd3c18674a102058aa8392a43c48f4123ae
// SCE-MAP: algorithm_ledger_total.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `ledger_total`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package ledger_total

import (
	scealgorithm "github.com/newmassrael/sce-forge-runtime/algorithm"
	sce_schema_entry "github.com/newmassrael/sce-go-tests/integration/static_datamodel/schema_entry"
	sce_schema_total "github.com/newmassrael/sce-go-tests/integration/static_datamodel/schema_total"
)

func LedgerTotal(entries []sce_schema_entry.SchemaEntryPayload) (sceValue sce_schema_total.SchemaTotalPayload, sceErr error) {
    // SCE_FORGE.md §3.4.1: each checked operation records a failure here, and
    // the statement around it returns it before the next statement runs.
    var sceFailure scealgorithm.Failure
    t := sce_schema_total.SchemaTotalPayload{Count: 0, Sum: 0}
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    for _, e := range entries {
        t.Count = scealgorithm.AddUint32(&sceFailure, t.Count, 1);
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        t.Sum = scealgorithm.AddInt32(&sceFailure, t.Sum, e.Amount);
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
    }
    {
        var sceReturned sce_schema_total.SchemaTotalPayload = t
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        return sceReturned, nil
    }
}
