// SCE-GENERATED — DO NOT EDIT
// source-hash: b8448cd407b1027864e71ab2f9c10d7e6a28146f7cc75313b45ffc4e7287b0f4
// SCE-MAP: orset_union.scxml:23 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.orset_union`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.orset_union

import com.sce.generated.orset_entry.*

fun orsetUnion(a: List<OrsetEntryPayload>, b: List<OrsetEntryPayload>): com.sce.forge.runtime.AlgorithmResult<List<OrsetEntryPayload>> {
    // SCE_FORGE.md §3.4.1: a checked operation throws its failure, and this
    // boundary hands it to the caller as a value — never as an exception.
    try {
        val out = ArrayList<OrsetEntryPayload>(1024)
        for (x in a) {
            out.add(x)
        }
        for (y in b) {
            var held: Boolean = false
            for (z in a) {
                if (z.element == y.element && z.wallTime == y.wallTime && z.counter == y.counter && z.nodeId == y.nodeId) {
                    held = true
                }
            }
            if (!held) {
                out.add(y)
            }
        }
        return com.sce.forge.runtime.AlgorithmResult.Ok(out)
    } catch (failure: com.sce.forge.runtime.AlgorithmFailure) {
        return com.sce.forge.runtime.AlgorithmResult.Failed(failure.error)
    }
}
