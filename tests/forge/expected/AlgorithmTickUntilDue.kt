// SCE-MAP: algorithm_tick_until_due:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.algorithm_tick_until_due`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.algorithm_tick_until_due

fun algorithmTickUntilDue(now: UInt, last: UInt, period: UInt): UInt {
    return com.sce.forge.runtime.SceSaturating.sub(period, com.sce.forge.runtime.SceWrapping.sub(now, last))
}
