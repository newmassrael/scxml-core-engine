// SCE-MAP: algorithm_cobs_encode:32

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.algorithm_cobs_encode`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.algorithm_cobs_encode

fun algorithmCobsEncode(data: ByteArray): ByteArray {
    var n: UShort = (data).size.toUShort()
    val out = mutableListOf<Byte>()
    var p: UShort = 0.toUShort()
    var done: Boolean = false
    while (done == false) {
        var q: UShort = p
        while (q < n && (q.toUInt() - p.toUInt()).toUShort() < 254.toUShort() && data[q.toInt()].toUByte() != 0.toUByte()) {
            q = (q.toUInt() + 1.toUInt()).toUShort()
        }
        var run: UShort = (q.toUInt() - p.toUInt()).toUShort()
        var code: UByte = (run.toUInt() + 1.toUInt()).toUByte()
        out.add((code).toByte())
        var k: UShort = p
        while (k < q) {
            out.add((data[k.toInt()].toUByte()).toByte())
            k = (k.toUInt() + 1.toUInt()).toUShort()
        }
        if (q >= n) {
            done = true
        } else {
            if (run < 254.toUShort()) {
                p = (q.toUInt() + 1.toUInt()).toUShort()
                if (p >= n) {
                    var last: UByte = 1.toUByte()
                    out.add((last).toByte())
                    done = true
                }
            } else {
                p = q
            }
        }
    }
    return out.toByteArray()
}
