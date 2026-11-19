// SCE-GENERATED — DO NOT EDIT
// source-hash: 8efa6e5bdb979d7a72d99505a1b8e8df1cfdca1e9dcbc9650c45b201c9ec78ec
// source: sce:std/text/text_slot_retitled.scxml
// SCE-MAP: text_slot_retitled.scxml:16 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.text_slot_retitled`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.text_slot_retitled

import com.sce.generated.text_slot.*

fun textSlotRetitled(slots: List<TextSlotPayload>): com.sce.forge.runtime.AlgorithmResult<List<TextSlotPayload>> {
    // SCE_FORGE.md §3.4.1: a checked operation throws its failure, and this
    // boundary hands it to the caller as a value — never as an exception.
    try {
        val out = ArrayList<TextSlotPayload>(16)
        for (s in slots) {
            var shown = TextSlotPayload(slot = s.slot, version = s.version, text = s.text)
            if (s.text == "") {
                shown = shown.copy(text = "untitled")
            }
            out.add(shown)
        }
        return com.sce.forge.runtime.AlgorithmResult.Ok(out)
    } catch (failure: com.sce.forge.runtime.AlgorithmFailure) {
        return com.sce.forge.runtime.AlgorithmResult.Failed(failure.error)
    }
}
