// SCE-MAP: algorithm_bytes_equal:18

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.bytes_equal`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.bytes_equal

fun bytesEqual(a: ByteArray, b: ByteArray): Boolean {
    if ((a).size != (b).size) {
        return false
    }
    var i: UInt = 0.toUInt()
    while (i < (a).size.toUInt()) {
        if (a[i.toInt()].toUByte() != b[i.toInt()].toUByte()) {
            return false
        }
        i = i + 1.toUInt()
    }
    return true
}
