// SCE-GENERATED — DO NOT EDIT
// source-hash: 2073d8a55462e1ce61d8a77293fdd2c141aceda8ad173303c5b84813ca22eeee
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
