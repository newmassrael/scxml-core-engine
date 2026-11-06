// SCE-GENERATED — DO NOT EDIT
// source-hash: 9aa28cb8a6152625fb0eb1560e8ab3014da5864745e85b6f3b6e37672c04dcf6
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
