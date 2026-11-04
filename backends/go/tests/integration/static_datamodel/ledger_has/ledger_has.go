// SCE-GENERATED — DO NOT EDIT
// source-hash: 8de0fa86739ee80dd1cde104a3417a50f438f9eccff41679327b28a2157e9381
// SCE-MAP: algorithm_ledger_has.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `ledger_has`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package ledger_has

import (
	sce_schema_entry "github.com/newmassrael/sce-go-tests/integration/static_datamodel/schema_entry"
)

func LedgerHas(entries []sce_schema_entry.SchemaEntryPayload, id uint32) bool {
    var found bool = false
    for _, e := range entries {
        if (e.Id == id) {
            found = true;
        }
    }
    return found
}
