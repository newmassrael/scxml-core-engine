// SCE-GENERATED — DO NOT EDIT
// source-hash: a58f6624548b520a5ba51086e8783c160d80754eb4040e27c0302b3da2a2c079
// source: outbound_overflows.scxml
// SCE-MAP: outbound_overflows.scxml:17 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.outbound_overflows`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.outbound_overflows

fun outboundOverflows(depth: UInt, maxPending: UInt): Boolean {
    return depth >= maxPending
}
