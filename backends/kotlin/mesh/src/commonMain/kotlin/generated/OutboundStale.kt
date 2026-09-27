// SCE-GENERATED — DO NOT EDIT
// source-hash: f6148b213f9ed141b381c5b54aabc8b1a585f1b96413497e91bf4f50d5cda1ce
// SCE-MAP: outbound_stale.scxml:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.outbound_stale`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.outbound_stale

fun outboundStale(enqueuedAtMs: Long, now: Long, maxAgeMs: Long): com.sce.forge.runtime.AlgorithmResult<Boolean> {
    // SCE_FORGE.md §3.4.1: a checked operation throws its failure, and this
    // boundary hands it to the caller as a value — never as an exception.
    try {
        var stale: Boolean = false
        if (maxAgeMs > 0) {
            stale = com.sce.forge.runtime.SceChecked.sub(now, enqueuedAtMs) > maxAgeMs
        }
        return com.sce.forge.runtime.AlgorithmResult.Ok(stale)
    } catch (failure: com.sce.forge.runtime.AlgorithmFailure) {
        return com.sce.forge.runtime.AlgorithmResult.Failed(failure.error)
    }
}
