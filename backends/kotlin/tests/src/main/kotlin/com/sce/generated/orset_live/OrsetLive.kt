// SCE-GENERATED — DO NOT EDIT
// source-hash: 58e52c40de99a046030dfe178fb69ffa932b0a6a8146307d7430d1e6de80a470
// SCE-MAP: orset_live.scxml:31 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.orset_live`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.orset_live

import com.sce.generated.orset_entry.*

fun orsetLive(adds: List<OrsetEntryPayload>, tombstones: List<OrsetEntryPayload>): com.sce.forge.runtime.AlgorithmResult<List<OrsetEntryPayload>> {
    // SCE_FORGE.md §3.4.1: a checked operation throws its failure, and this
    // boundary hands it to the caller as a value — never as an exception.
    try {
        val out = ArrayList<OrsetEntryPayload>(256)
        for (x in adds) {
            var removed: Boolean = false
            for (t in tombstones) {
                if (t.element == x.element && t.wallTime == x.wallTime && t.counter == x.counter && t.nodeId == x.nodeId) {
                    removed = true
                }
            }
            if (!removed) {
                out.add(x)
            }
        }
        return com.sce.forge.runtime.AlgorithmResult.Ok(out)
    } catch (failure: com.sce.forge.runtime.AlgorithmFailure) {
        return com.sce.forge.runtime.AlgorithmResult.Failed(failure.error)
    }
}
