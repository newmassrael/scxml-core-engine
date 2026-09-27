// SCE-GENERATED — DO NOT EDIT
// source-hash: ab119d19c373fb9e83e74bd30f74186a9e2f87ef70ba045b5c5ab8bb9e9d1849
// SCE-MAP: retry_jittered.scxml:23 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.retry_jittered`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.retry_jittered

fun retryJittered(baseMs: Long, jitterPct: Long, draw: Long): com.sce.forge.runtime.AlgorithmResult<Long> {
    // SCE_FORGE.md §3.4.1: a checked operation throws its failure, and this
    // boundary hands it to the caller as a value — never as an exception.
    try {
        if (!(baseMs > 0 && jitterPct >= 0 && jitterPct <= 100 && draw >= 0)) {
            return com.sce.forge.runtime.AlgorithmResult.Failed(com.sce.forge.runtime.AlgorithmError.Precondition)
        }
        var delta: Long = com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(com.sce.forge.runtime.SceChecked.div(baseMs, 100), jitterPct), com.sce.forge.runtime.SceChecked.div(com.sce.forge.runtime.SceChecked.mul(com.sce.forge.runtime.SceChecked.rem(baseMs, 100), jitterPct), 100))
        var waited: Long = com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.sub(baseMs, delta), com.sce.forge.runtime.SceChecked.rem(draw, com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(2, delta), 1)))
        return com.sce.forge.runtime.AlgorithmResult.Ok(if (waited < 1) 1 else waited)
    } catch (failure: com.sce.forge.runtime.AlgorithmFailure) {
        return com.sce.forge.runtime.AlgorithmResult.Failed(failure.error)
    }
}
