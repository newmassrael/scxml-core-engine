// SCE-GENERATED — DO NOT EDIT
// source-hash: 299edcd86435fd0aaceb858143fb6e6689d45f9269775736c7c99fe0feb97e4c
// SCE-MAP: algorithm_ledger_bump.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.ledger_bump`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.ledger_bump

import com.sce.generated.schema_total.*

fun ledgerBump(total: SchemaTotalPayload, delta: Int): com.sce.forge.runtime.AlgorithmResult<SchemaTotalPayload> {
    // SCE_FORGE.md §3.4.1: a checked operation throws its failure, and this
    // boundary hands it to the caller as a value — never as an exception.
    try {
        var next = SchemaTotalPayload(count = com.sce.forge.runtime.SceChecked.add(total.count, 1.toUInt()), sum = com.sce.forge.runtime.SceChecked.add(total.sum, delta))
        return com.sce.forge.runtime.AlgorithmResult.Ok(next)
    } catch (failure: com.sce.forge.runtime.AlgorithmFailure) {
        return com.sce.forge.runtime.AlgorithmResult.Failed(failure.error)
    }
}
