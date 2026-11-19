// SCE-GENERATED — DO NOT EDIT
// source-hash: 62e099c55c3af43d1033820a3493a3d8ff5082ff405738e08864fc63e2adbb84
// source: sce:std/text/text_slot_holds.scxml
// SCE-MAP: text_slot_holds.scxml:14 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.text_slot_holds`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.text_slot_holds

import com.sce.generated.text_slot.*

fun textSlotHolds(slots: List<TextSlotPayload>, probe: TextSlotPayload): Boolean {
    var found: Boolean = false
    for (s in slots) {
        if (s.slot == probe.slot && s.text == probe.text) {
            found = true
        }
    }
    return found
}
