// SCE-MAP: algorithm_const_fold_smoke:24

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.algorithm_const_fold_smoke`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.
@file:OptIn(kotlin.ExperimentalUnsignedTypes::class)

package com.sce.generated.algorithm_const_fold_smoke

val DOUBLED: UShortArray = ushortArrayOf((0).toUShort(), (2).toUShort(), (4).toUShort(), (6).toUShort())

fun algorithmConstFoldSmoke(): UShort {
    return 0.toUShort()
}
