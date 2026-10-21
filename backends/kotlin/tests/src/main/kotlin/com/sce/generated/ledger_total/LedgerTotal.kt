// SCE-GENERATED — DO NOT EDIT
// source-hash: 9eefb6dd8cbce9cedf75b07dbf863e3e812c459c1ce7686309e1f4c36da06a10
// SCE-MAP: algorithm_ledger_total.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.ledger_total`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.ledger_total

import com.sce.generated.schema_entry.*
import com.sce.generated.schema_total.*

fun ledgerTotal(entries: List<SchemaEntryPayload>): com.sce.forge.runtime.AlgorithmResult<SchemaTotalPayload> {
    // SCE_FORGE.md §3.4.1: a checked operation throws its failure, and this
    // boundary hands it to the caller as a value — never as an exception.
    try {
        var t = SchemaTotalPayload(count = 0.toUInt(), sum = 0)
        for (e in entries) {
            t = t.copy(count = com.sce.forge.runtime.SceChecked.add(t.count, 1.toUInt()))
            t = t.copy(sum = com.sce.forge.runtime.SceChecked.add(t.sum, e.amount))
        }
        return com.sce.forge.runtime.AlgorithmResult.Ok(t)
    } catch (failure: com.sce.forge.runtime.AlgorithmFailure) {
        return com.sce.forge.runtime.AlgorithmResult.Failed(failure.error)
    }
}
