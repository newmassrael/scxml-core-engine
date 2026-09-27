// SCE-GENERATED — DO NOT EDIT
// source-hash: f6148b213f9ed141b381c5b54aabc8b1a585f1b96413497e91bf4f50d5cda1ce
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
