// SCE-GENERATED — DO NOT EDIT
// source-hash: bb79893ae8ff05aa98d88701952b9689dfd413403f164f7a9bab8d73d125746f
// source: algorithm_slot_titled.scxml
// SCE-MAP: algorithm_slot_titled.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.slot_titled`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.slot_titled

import com.sce.generated.text_slot.*

fun slotTitled(title: String, slots: List<TextSlotPayload>): Boolean {
    var found: Boolean = false
    for (s in slots) {
        if (s.text == title) {
            found = true
        }
    }
    return found
}
