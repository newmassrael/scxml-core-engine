// SCE-GENERATED — DO NOT EDIT
// source-hash: c0802ab29f5b9eace062e478a717aaf037a02f7f1368f1ee6542066106d79c77
// SCE-MAP: sync_failure.scxml:30 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.sync_failure`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.sync_failure

fun syncFailure(kind: UByte, status: Int): com.sce.forge.runtime.AlgorithmResult<UByte> {
    // SCE_FORGE.md §3.4.1: a checked operation throws its failure, and this
    // boundary hands it to the caller as a value — never as an exception.
    try {
        if (!(kind >= 1.toUByte() && kind <= 9.toUByte())) {
            return com.sce.forge.runtime.AlgorithmResult.Failed(com.sce.forge.runtime.AlgorithmError.Precondition)
        }
        if (!(if (kind == 4.toUByte()) status >= 300 && status <= 599 else status == 0)) {
            return com.sce.forge.runtime.AlgorithmResult.Failed(com.sce.forge.runtime.AlgorithmError.Precondition)
        }
        var action: UByte = 2.toUByte()
        if (kind == 8.toUByte() || kind == 1.toUByte() || kind == 2.toUByte()) {
            action = 1.toUByte()
        }
        if (kind == 7.toUByte()) {
            action = 4.toUByte()
        }
        if (kind == 3.toUByte()) {
            action = 0.toUByte()
        }
        if (kind == 4.toUByte() && status == 401) {
            action = 3.toUByte()
        }
        if (kind == 4.toUByte() && (status == 503 || status == 502)) {
            action = 1.toUByte()
        }
        return com.sce.forge.runtime.AlgorithmResult.Ok(action)
    } catch (failure: com.sce.forge.runtime.AlgorithmFailure) {
        return com.sce.forge.runtime.AlgorithmResult.Failed(failure.error)
    }
}
