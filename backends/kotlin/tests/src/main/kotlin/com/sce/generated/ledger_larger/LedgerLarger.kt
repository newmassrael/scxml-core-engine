// SCE-GENERATED — DO NOT EDIT
// source-hash: 32f3d7c11ac80be29d57e43f3bd96bf8c60dfd06eee3d5e4898056eaf3848397
// source: algorithm_ledger_larger.scxml
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
