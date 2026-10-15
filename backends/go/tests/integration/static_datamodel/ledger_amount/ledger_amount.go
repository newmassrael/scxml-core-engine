// SCE-GENERATED — DO NOT EDIT
// source-hash: 41d1f204b82750ba4d9d73e8625511079107366428dbecb4676ad93e594975ae
// SCE-MAP: algorithm_ledger_amount.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `ledger_amount`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package ledger_amount

import (
	sce_schema_entry "github.com/newmassrael/sce-go-tests/integration/static_datamodel/schema_entry"
)

func LedgerAmount(line sce_schema_entry.SchemaEntryPayload) int32 {
    return line.Amount
}
