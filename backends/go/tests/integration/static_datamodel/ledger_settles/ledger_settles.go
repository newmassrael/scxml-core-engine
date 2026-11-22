// SCE-GENERATED — DO NOT EDIT
// source-hash: da008e408cd55b290322e8155517ad19a85661d68eef0def5f8178d4e21288f7
// source: algorithm_ledger_settles.scxml
// SCE-MAP: algorithm_ledger_settles.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `ledger_settles`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package ledger_settles

import (
	sce_enum_item_status "github.com/newmassrael/sce-go-tests/integration/static_datamodel/enum_item_status"
)

func LedgerSettles(status sce_enum_item_status.EnumItemStatus, amount int32) bool {
    return status == sce_enum_item_status.EnumItemStatusDone && amount > 0
}
