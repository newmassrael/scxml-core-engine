// SCE-GENERATED — DO NOT EDIT
// source-hash: 68d77b5e0c96e361239e1512887ba2ef6d35bfab0188bf0e2afbd53ca387f3bc
// SCE-MAP: utc_offset_at.scxml:25 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.utc_offset_at`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.utc_offset_at

fun utcOffsetAt(starts: LongArray, offsets: IntArray, t: Long): com.sce.forge.runtime.AlgorithmResult<Int> {
    // SCE_FORGE.md §3.4.1: a checked operation throws its failure, and this
    // boundary hands it to the caller as a value — never as an exception.
    try {
        if (!((starts).size == (offsets).size && (starts).size > 0)) {
            return com.sce.forge.runtime.AlgorithmResult.Failed(com.sce.forge.runtime.AlgorithmError.Precondition)
        }
        if (!(t >= com.sce.forge.runtime.SceChecked.at(starts, (0).toLong()))) {
            return com.sce.forge.runtime.AlgorithmResult.Failed(com.sce.forge.runtime.AlgorithmError.Precondition)
        }
        var k: UInt = 0.toUInt()
        var i: UInt = 1.toUInt()
        while (i < (starts).size.toUInt()) {
            if (!(com.sce.forge.runtime.SceChecked.at(starts, (i).toLong()) > com.sce.forge.runtime.SceChecked.at(starts, (com.sce.forge.runtime.SceChecked.sub(i, 1.toUInt())).toLong()))) {
                return com.sce.forge.runtime.AlgorithmResult.Failed(com.sce.forge.runtime.AlgorithmError.Precondition)
            }
            if (com.sce.forge.runtime.SceChecked.at(starts, (i).toLong()) <= t) {
                k = i
            }
            i = com.sce.forge.runtime.SceChecked.add(i, 1.toUInt())
        }
        return com.sce.forge.runtime.AlgorithmResult.Ok(com.sce.forge.runtime.SceChecked.at(offsets, (k).toLong()))
    } catch (failure: com.sce.forge.runtime.AlgorithmFailure) {
        return com.sce.forge.runtime.AlgorithmResult.Failed(failure.error)
    }
}
