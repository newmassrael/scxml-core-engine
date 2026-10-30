// SCE-MAP: algorithm_lcg_table:13 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.algorithm_lcg_table`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.
@file:OptIn(kotlin.ExperimentalUnsignedTypes::class)

package com.sce.generated.algorithm_lcg_table

val LCG_TABLE: UIntArray = uintArrayOf(12345u, 87628868u, 71072467u, 2332836374u, 2726892157u, 3908547000u, 483019191u, 2129828778u, 2355140353u, 2560230508u, 3364893915u, 171172990u, 3194601925u, 4148119648u, 316399679u, 3004788882u)

fun algorithmLcgTable(n: UByte): UInt {
    return LCG_TABLE[n.toInt() and 15]
}
