// SCE-GENERATED — DO NOT EDIT
// source-hash: 9ad3991e659415a87e89be4422403fddaea9105e2a522b988eeacca0944ca255
// SCE-MAP: hlc_receive.scxml:19 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.hlc_receive`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.hlc_receive

import com.sce.generated.hlc_timestamp.*

fun hlcReceive(prev: HlcTimestampPayload, msg: HlcTimestampPayload, now: Long, node: ULong): com.sce.forge.runtime.AlgorithmResult<HlcTimestampPayload> {
    // SCE_FORGE.md §3.4.1: a checked operation throws its failure, and this
    // boundary hands it to the caller as a value — never as an exception.
    try {
        var seen: Long = if (prev.wallTime > msg.wallTime) prev.wallTime else msg.wallTime
        var next = HlcTimestampPayload(wallTime = if (now > seen) now else seen, counter = 0.toUInt(), nodeId = node)
        if (next.wallTime == prev.wallTime && next.wallTime == msg.wallTime) {
            next = next.copy(counter = com.sce.forge.runtime.SceChecked.add(if (prev.counter > msg.counter) prev.counter else msg.counter, 1.toUInt()))
        } else {
            if (next.wallTime == prev.wallTime) {
                next = next.copy(counter = com.sce.forge.runtime.SceChecked.add(prev.counter, 1.toUInt()))
            } else {
                if (next.wallTime == msg.wallTime) {
                    next = next.copy(counter = com.sce.forge.runtime.SceChecked.add(msg.counter, 1.toUInt()))
                }
            }
        }
        return com.sce.forge.runtime.AlgorithmResult.Ok(next)
    } catch (failure: com.sce.forge.runtime.AlgorithmFailure) {
        return com.sce.forge.runtime.AlgorithmResult.Failed(failure.error)
    }
}
