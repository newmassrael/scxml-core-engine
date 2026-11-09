// SCE-GENERATED — DO NOT EDIT
// source-hash: d05ce240c5f3b48b3c83b6bee1ba58feef40d57c310e9a29ef5279604bbaaa42
// SCE-MAP: algorithm_ledger_larger.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.ledger_larger`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.ledger_larger

import com.sce.generated.schema_entry.*

fun ledgerLarger(a: SchemaEntryPayload, b: SchemaEntryPayload): SchemaEntryPayload {
    var out = SchemaEntryPayload(id = a.id, amount = a.amount)
    if (b.amount > a.amount) {
        out = out.copy(id = b.id)
        out = out.copy(amount = b.amount)
    }
    return out
}
