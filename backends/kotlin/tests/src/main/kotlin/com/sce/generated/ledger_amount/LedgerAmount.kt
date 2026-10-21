// SCE-GENERATED — DO NOT EDIT
// source-hash: 048fe3b13b4ba344e3f60c0d3a93bf8f8568ab7bd254b4d4763d7e631895e2f6
// SCE-MAP: algorithm_ledger_amount.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.ledger_amount`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.ledger_amount

import com.sce.generated.schema_entry.*

fun ledgerAmount(line: SchemaEntryPayload): Int {
    return line.amount
}
