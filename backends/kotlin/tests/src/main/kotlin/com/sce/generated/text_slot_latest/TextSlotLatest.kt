// SCE-GENERATED — DO NOT EDIT
// source-hash: 848898b5bbb016bf799fdf5a1235f8e512e7516e53f20e55bfd5887716ace728
// SCE-MAP: text_slot_latest.scxml:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.text_slot_latest`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.text_slot_latest

import com.sce.generated.text_slot.*

fun textSlotLatest(a: TextSlotPayload, b: TextSlotPayload): TextSlotPayload {
    var out = TextSlotPayload(slot = a.slot, version = a.version, text = a.text)
    if (b.version > a.version) {
        out = out.copy(version = b.version)
        if (b.text != a.text) {
            out = out.copy(text = b.text)
        }
    }
    return out
}
