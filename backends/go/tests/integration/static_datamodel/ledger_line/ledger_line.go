// SCE-GENERATED — DO NOT EDIT
// source-hash: a4d51341b4c2faa9ecf7e674a095736d0b61e4dd0db21e43d561c13b65d4550d
// source: algorithm_ledger_line.scxml
// SCE-MAP: algorithm_ledger_line.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `ledger_line`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package ledger_line

import (
	sce_schema_entry "github.com/newmassrael/sce-go-tests/integration/static_datamodel/schema_entry"
)

func LedgerLine(id uint32, amount int32) sce_schema_entry.SchemaEntryPayload {
    out := sce_schema_entry.SchemaEntryPayload{Id: id, Amount: amount}
    return out
}
