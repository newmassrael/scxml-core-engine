// SCE-MAP: algorithm_crc16:11

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §5.A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.algorithm_crc16`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §5.J.5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.algorithm_crc16

fun algorithmCrc16(data: ByteArray): UShort {
    var crc: UShort = 0xFFFF.toUShort()
    for (__raw_b in data) {
        val b: UByte = __raw_b.toUByte()
        var hi: UShort = b.toUShort()
        crc = (crc.toInt() xor (hi.toInt() shl 8)).toUShort()
        var i: UByte = 0.toUByte()
        while (i < 8.toUByte()) {
            if ((crc.toInt() and 0x8000).toUShort() != 0.toUShort()) {
                crc = (crc.toInt() shl 1 xor 0x1021).toUShort()
            } else {
                crc = (crc.toInt() shl 1).toUShort()
            }
            i = (i.toUInt() + 1.toUInt()).toUByte()
        }
    }
    return crc
}
