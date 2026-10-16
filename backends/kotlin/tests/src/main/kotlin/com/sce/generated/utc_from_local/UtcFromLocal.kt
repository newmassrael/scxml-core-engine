// SCE-GENERATED — DO NOT EDIT
// source-hash: 68d77b5e0c96e361239e1512887ba2ef6d35bfab0188bf0e2afbd53ca387f3bc
// SCE-MAP: utc_from_local.scxml:30 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.utc_from_local`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.utc_from_local

fun utcFromLocal(starts: LongArray, offsets: IntArray, local: Long): com.sce.forge.runtime.AlgorithmResult<Long> {
    // SCE_FORGE.md §3.4.1: a checked operation throws its failure, and this
    // boundary hands it to the caller as a value — never as an exception.
    try {
        if (!((starts).size == (offsets).size && (starts).size > 0)) {
            return com.sce.forge.runtime.AlgorithmResult.Failed(com.sce.forge.runtime.AlgorithmError.Precondition)
        }
        if (!(com.sce.forge.runtime.SceChecked.sub(local, com.sce.forge.runtime.SceChecked.at(offsets, (0).toLong()).toLong()) >= com.sce.forge.runtime.SceChecked.at(starts, (0).toLong()))) {
            return com.sce.forge.runtime.AlgorithmResult.Failed(com.sce.forge.runtime.AlgorithmError.Precondition)
        }
        var n: UInt = (starts).size.toUInt()
        var k: UInt = 0.toUInt()
        while (k < n) {
            if (k > 0.toUInt()) {
                if (!(com.sce.forge.runtime.SceChecked.at(starts, (k).toLong()) > com.sce.forge.runtime.SceChecked.at(starts, (com.sce.forge.runtime.SceChecked.sub(k, 1.toUInt())).toLong()))) {
                    return com.sce.forge.runtime.AlgorithmResult.Failed(com.sce.forge.runtime.AlgorithmError.Precondition)
                }
            }
            var u: Long = com.sce.forge.runtime.SceChecked.sub(local, com.sce.forge.runtime.SceChecked.at(offsets, (k).toLong()).toLong())
            if (u >= com.sce.forge.runtime.SceChecked.at(starts, (k).toLong()) && (com.sce.forge.runtime.SceChecked.add(k, 1.toUInt()) == n || u < com.sce.forge.runtime.SceChecked.at(starts, (com.sce.forge.runtime.SceChecked.add(k, 1.toUInt())).toLong()))) {
                return com.sce.forge.runtime.AlgorithmResult.Ok(u)
            }
            k = com.sce.forge.runtime.SceChecked.add(k, 1.toUInt())
        }
        var g: UInt = 1.toUInt()
        while (g < n) {
            var before: Long = com.sce.forge.runtime.SceChecked.sub(local, com.sce.forge.runtime.SceChecked.at(offsets, (com.sce.forge.runtime.SceChecked.sub(g, 1.toUInt())).toLong()).toLong())
            if (before >= com.sce.forge.runtime.SceChecked.at(starts, (g).toLong()) && com.sce.forge.runtime.SceChecked.sub(local, com.sce.forge.runtime.SceChecked.at(offsets, (g).toLong()).toLong()) < com.sce.forge.runtime.SceChecked.at(starts, (g).toLong())) {
                return com.sce.forge.runtime.AlgorithmResult.Ok(before)
            }
            g = com.sce.forge.runtime.SceChecked.add(g, 1.toUInt())
        }
        if (!(false)) {
            return com.sce.forge.runtime.AlgorithmResult.Failed(com.sce.forge.runtime.AlgorithmError.Precondition)
        }
        return com.sce.forge.runtime.AlgorithmResult.Ok(0)
    } catch (failure: com.sce.forge.runtime.AlgorithmFailure) {
        return com.sce.forge.runtime.AlgorithmResult.Failed(failure.error)
    }
}
