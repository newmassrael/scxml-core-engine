// SCE-GENERATED — DO NOT EDIT
// source-hash: c431fa9e45990e8b729e69a4bf81ba13f9cc4e8066ddecbd5ff6346ac56dd5f3
// SCE-MAP: hlc_send.scxml:17 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.hlc_send`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.hlc_send

import com.sce.generated.hlc_timestamp.*

fun hlcSend(prev: HlcTimestampPayload, now: Long, node: ULong): com.sce.forge.runtime.AlgorithmResult<HlcTimestampPayload> {
    // SCE_FORGE.md §3.4.1: a checked operation throws its failure, and this
    // boundary hands it to the caller as a value — never as an exception.
    try {
        var next = HlcTimestampPayload(wallTime = if (now > prev.wallTime) now else prev.wallTime, counter = 0.toUInt(), nodeId = node)
        if (next.wallTime == prev.wallTime) {
            next = next.copy(counter = com.sce.forge.runtime.SceChecked.add(prev.counter, 1.toUInt()))
        }
        return com.sce.forge.runtime.AlgorithmResult.Ok(next)
    } catch (failure: com.sce.forge.runtime.AlgorithmFailure) {
        return com.sce.forge.runtime.AlgorithmResult.Failed(failure.error)
    }
}
