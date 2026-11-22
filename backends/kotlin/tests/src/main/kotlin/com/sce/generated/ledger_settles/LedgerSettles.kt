// SCE-GENERATED — DO NOT EDIT
// source-hash: da008e408cd55b290322e8155517ad19a85661d68eef0def5f8178d4e21288f7
// source: algorithm_ledger_settles.scxml
// SCE-MAP: algorithm_ledger_settles.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.ledger_settles`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.ledger_settles

import com.sce.generated.enum_item_status.*

fun ledgerSettles(status: EnumItemStatus, amount: Int): Boolean {
    return status == EnumItemStatus.DONE && amount > 0
}
