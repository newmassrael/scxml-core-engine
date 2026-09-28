// SCE-GENERATED — DO NOT EDIT
// source-hash: c0802ab29f5b9eace062e478a717aaf037a02f7f1368f1ee6542066106d79c77
// SCE-MAP: sync_retry_at.scxml:28 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.sync_retry_at`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.sync_retry_at

fun syncRetryAt(previous: Long, kind: UByte, status: Int, now: Long, retryAfter: Long): com.sce.forge.runtime.AlgorithmResult<Long> {
    // SCE_FORGE.md §3.4.1: a checked operation throws its failure, and this
    // boundary hands it to the caller as a value — never as an exception.
    try {
        if (!(kind >= 1.toUByte() && kind <= 9.toUByte())) {
            return com.sce.forge.runtime.AlgorithmResult.Failed(com.sce.forge.runtime.AlgorithmError.Precondition)
        }
        if (!(if (kind == 4.toUByte()) status >= 300 && status <= 599 else status == 0)) {
            return com.sce.forge.runtime.AlgorithmResult.Failed(com.sce.forge.runtime.AlgorithmError.Precondition)
        }
        if (!(previous >= 0 && now >= 0 && retryAfter >= 0)) {
            return com.sce.forge.runtime.AlgorithmResult.Failed(com.sce.forge.runtime.AlgorithmError.Precondition)
        }
        var asked: Long = 0
        if (kind == 4.toUByte() && status == 503 && retryAfter > 0) {
            asked = com.sce.forge.runtime.SceChecked.add(now, retryAfter)
        }
        if (kind == 4.toUByte() && status == 502) {
            asked = com.sce.forge.runtime.SceChecked.add(now, 900)
        }
        return com.sce.forge.runtime.AlgorithmResult.Ok(if (asked > previous) asked else previous)
    } catch (failure: com.sce.forge.runtime.AlgorithmFailure) {
        return com.sce.forge.runtime.AlgorithmResult.Failed(failure.error)
    }
}
