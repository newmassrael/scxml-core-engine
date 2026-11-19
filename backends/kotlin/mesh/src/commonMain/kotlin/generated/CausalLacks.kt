// SCE-GENERATED — DO NOT EDIT
// source-hash: d3495fb62176332a10644d56c21a8821d7d8d4139f8e29b449e0d708325991a4
// source: causal_lacks.scxml
// SCE-MAP: causal_lacks.scxml:16 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.causal_lacks`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.causal_lacks

import com.sce.generated.causal_entry.*

fun causalLacks(vector: List<CausalEntryPayload>, dot: CausalEntryPayload): Boolean {
    var seen: ULong = 0.toULong()
    for (entry in vector) {
        if (entry.hi == dot.hi && entry.lo == dot.lo) {
            seen = entry.seq
        }
    }
    return dot.seq > seen
}
