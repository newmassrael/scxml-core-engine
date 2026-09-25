// SCE-MAP: algorithm_checked_arith:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.algorithm_checked_arith`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.algorithm_checked_arith

fun algorithmCheckedArith(a: Int, b: Int, op: UByte): com.sce.forge.runtime.AlgorithmResult<Int> {
    // SCE_FORGE.md §3.4.1: a checked operation throws its failure, and this
    // boundary hands it to the caller as a value — never as an exception.
    try {
        var r: Int = 0
        if (op == 0.toUByte()) {
            r = com.sce.forge.runtime.SceChecked.add(a, b)
        }
        if (op == 1.toUByte()) {
            r = com.sce.forge.runtime.SceChecked.sub(a, b)
        }
        if (op == 2.toUByte()) {
            r = com.sce.forge.runtime.SceChecked.mul(a, b)
        }
        if (op == 3.toUByte()) {
            r = com.sce.forge.runtime.SceChecked.div(a, b)
        }
        if (op == 4.toUByte()) {
            r = com.sce.forge.runtime.SceChecked.rem(a, b)
        }
        if (op == 5.toUByte()) {
            r = com.sce.forge.runtime.SceChecked.neg(a)
        }
        if (op == 6.toUByte()) {
            r = com.sce.forge.runtime.SceChecked.sub(op, 7.toUByte()).toInt()
        }
        return com.sce.forge.runtime.AlgorithmResult.Ok(r)
    } catch (failure: com.sce.forge.runtime.AlgorithmFailure) {
        return com.sce.forge.runtime.AlgorithmResult.Failed(failure.error)
    }
}
