// SCE-GENERATED — DO NOT EDIT
// source-hash: a92737f3147f50ca2cf6319055f29d7afe2c17766723037ae46a81e73e7a66df
// SCE-MAP: algorithm_ledger_add.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `ledger_add`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package ledger_add

import (
	scealgorithm "github.com/newmassrael/sce-forge-runtime/algorithm"
	sce_schema_entry "github.com/newmassrael/sce-go-tests/integration/static_datamodel/schema_entry"
)

func LedgerAdd(entries []sce_schema_entry.SchemaEntryPayload, id uint32, amount int32) (sceValue []sce_schema_entry.SchemaEntryPayload, sceErr error) {
    // SCE_FORGE.md §3.4.1: each checked operation records a failure here, and
    // the statement around it returns it before the next statement runs.
    var sceFailure scealgorithm.Failure
    fresh := sce_schema_entry.SchemaEntryPayload{Id: id, Amount: amount}
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    out := make([]sce_schema_entry.SchemaEntryPayload, 0, 8)
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var seen bool = false
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    for _, e := range entries {
        if sceCond2 := e.Id == id; sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        } else if sceCond2 {
            seen = true;
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
            out = append(out, fresh)
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
    if sceCond1 := !seen; sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        out = append(out, fresh)
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
    }
    {
        var sceReturned []sce_schema_entry.SchemaEntryPayload = out
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        return sceReturned, nil
    }
}
