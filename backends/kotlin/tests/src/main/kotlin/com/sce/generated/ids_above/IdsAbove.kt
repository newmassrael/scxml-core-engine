// SCE-GENERATED — DO NOT EDIT
// source-hash: 43faded87b84ec06f387d0e3d6ccb626e2fa1802d2395c65b70588a4c3c394af
// SCE-MAP: algorithm_ids_above.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.ids_above`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.
@file:OptIn(kotlin.ExperimentalUnsignedTypes::class)

package com.sce.generated.ids_above

fun idsAbove(ids: UIntArray, floor: UInt): Boolean {
    var above: Boolean = false
    for (id in ids) {
        if (id > floor) {
            above = true
        }
    }
    return above
}
