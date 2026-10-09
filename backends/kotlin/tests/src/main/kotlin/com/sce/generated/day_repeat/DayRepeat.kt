// SCE-GENERATED — DO NOT EDIT
// source-hash: cb41954d893211ff980559eb7566d5cfca26a8d312a941a8b0426661b4fb8dcd
// SCE-MAP: algorithm_day_repeat.scxml:13 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.day_repeat`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.
@file:OptIn(kotlin.ExperimentalUnsignedTypes::class)

package com.sce.generated.day_repeat

import com.sce.forge.runtime.SceListBuf

fun dayRepeat(value: UByte, count: UByte): com.sce.forge.runtime.AlgorithmResult<UByteArray> {
    // SCE_FORGE.md §3.4.1: a checked operation throws its failure, and this
    // boundary hands it to the caller as a value — never as an exception.
    try {
        val out = SceListBuf(4)
        var i: UByte = 0.toUByte()
        while (i < count) {
            out.add(value)
            i = com.sce.forge.runtime.SceChecked.add(i, 1.toUByte())
        }
        return com.sce.forge.runtime.AlgorithmResult.Ok(out.toUByteArray())
    } catch (failure: com.sce.forge.runtime.AlgorithmFailure) {
        return com.sce.forge.runtime.AlgorithmResult.Failed(failure.error)
    }
}
