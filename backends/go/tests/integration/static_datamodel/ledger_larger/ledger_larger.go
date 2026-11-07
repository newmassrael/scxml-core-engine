// SCE-GENERATED — DO NOT EDIT
// source-hash: b2921c03007062914a1e69cc4069c6b54c8d66f97313a50f3246eed3d071c790
// SCE-MAP: algorithm_ledger_larger.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `ledger_larger`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package ledger_larger

import (
	sce_schema_entry "github.com/newmassrael/sce-go-tests/integration/static_datamodel/schema_entry"
)

func LedgerLarger(a sce_schema_entry.SchemaEntryPayload, b sce_schema_entry.SchemaEntryPayload) sce_schema_entry.SchemaEntryPayload {
    out := sce_schema_entry.SchemaEntryPayload{Id: a.Id, Amount: a.Amount}
    if (b.Amount > a.Amount) {
        out.Id = b.Id;
        out.Amount = b.Amount;
    }
    return out
}
