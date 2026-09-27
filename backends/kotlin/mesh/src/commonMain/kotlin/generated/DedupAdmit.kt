// SCE-GENERATED — DO NOT EDIT
// source-hash: ab119d19c373fb9e83e74bd30f74186a9e2f87ef70ba045b5c5ab8bb9e9d1849
// SCE-MAP: dedup_admit.scxml:26 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.dedup_admit`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.dedup_admit

import com.sce.generated.envelope_id.*

fun dedupAdmit(window: List<EnvelopeIdPayload>, id: EnvelopeIdPayload, capacity: UInt): com.sce.forge.runtime.AlgorithmResult<List<EnvelopeIdPayload>> {
    // SCE_FORGE.md §3.4.1: a checked operation throws its failure, and this
    // boundary hands it to the caller as a value — never as an exception.
    try {
        if (!(capacity > 0.toUInt() && (window).size.toUInt() <= capacity)) {
            return com.sce.forge.runtime.AlgorithmResult.Failed(com.sce.forge.runtime.AlgorithmError.Precondition)
        }
        var held: Boolean = false
        for (x in window) {
            if (x.hi == id.hi && x.lo == id.lo) {
                held = true
            }
        }
        val out = ArrayList<EnvelopeIdPayload>(256)
        var skip: UInt = (if (held || (window).size.toUInt() < capacity) 0 else 1).toUInt()
        var i: UInt = 0.toUInt()
        for (kept in window) {
            if (i >= skip) {
                out.add(kept)
            }
            i = com.sce.forge.runtime.SceChecked.add(i, 1.toUInt())
        }
        if (!held) {
            out.add(id)
        }
        return com.sce.forge.runtime.AlgorithmResult.Ok(out)
    } catch (failure: com.sce.forge.runtime.AlgorithmFailure) {
        return com.sce.forge.runtime.AlgorithmResult.Failed(failure.error)
    }
}
