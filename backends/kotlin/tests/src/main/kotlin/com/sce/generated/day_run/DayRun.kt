// SCE-GENERATED — DO NOT EDIT
// source-hash: 7ad55f268a9fbf4c094293a60d20a17c7e9e6598a9a6fe5787c0e9f2898a7e28
// SCE-MAP: algorithm_day_run.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.day_run`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.
@file:OptIn(kotlin.ExperimentalUnsignedTypes::class)

package com.sce.generated.day_run

import com.sce.forge.runtime.SceListBuf

fun dayRun(first: UByte, count: UByte): com.sce.forge.runtime.AlgorithmResult<UByteArray> {
    // SCE_FORGE.md §3.4.1: a checked operation throws its failure, and this
    // boundary hands it to the caller as a value — never as an exception.
    try {
        val out = SceListBuf(8)
        var i: UByte = 0.toUByte()
        while (i < count) {
            out.add(com.sce.forge.runtime.SceChecked.add(first, i))
            i = com.sce.forge.runtime.SceChecked.add(i, 1.toUByte())
        }
        return com.sce.forge.runtime.AlgorithmResult.Ok(out.toUByteArray())
    } catch (failure: com.sce.forge.runtime.AlgorithmFailure) {
        return com.sce.forge.runtime.AlgorithmResult.Failed(failure.error)
    }
}
