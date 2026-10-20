// SCE-GENERATED — DO NOT EDIT
// source-hash: b3df842d016676bf8989832cb164de51027fadf6ca9e7b5543c7bac0c9a13d1a
// SCE-MAP: rga_insert.scxml:25 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.rga_insert`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.rga_insert

import com.sce.generated.rga_node.*

fun rgaInsert(nodes: List<RgaNodePayload>, node: RgaNodePayload): com.sce.forge.runtime.AlgorithmResult<List<RgaNodePayload>> {
    // SCE_FORGE.md §3.4.1: a checked operation throws its failure, and this
    // boundary hands it to the caller as a value — never as an exception.
    try {
        val out = ArrayList<RgaNodePayload>(256)
        var held: Boolean = false
        for (probe in nodes) {
            if (probe.key == node.key) {
                held = true
            }
        }
        if (held) {
            for (same in nodes) {
                out.add(same)
            }
            return com.sce.forge.runtime.AlgorithmResult.Ok(out)
        }
        var phase: UByte = (if (node.after == 0.toULong()) 1 else 0).toUByte()
        for (n in nodes) {
            if (phase == 0.toUByte()) {
                out.add(n)
                if (n.key == node.after) {
                    phase = 1.toUByte()
                }
            } else {
                if (phase == 1.toUByte()) {
                    if (n.key > node.key) {
                        out.add(n)
                    } else {
                        out.add(node)
                        out.add(n)
                        phase = 2.toUByte()
                    }
                } else {
                    out.add(n)
                }
            }
        }
        if (phase != 2.toUByte()) {
            out.add(node)
        }
        return com.sce.forge.runtime.AlgorithmResult.Ok(out)
    } catch (failure: com.sce.forge.runtime.AlgorithmFailure) {
        return com.sce.forge.runtime.AlgorithmResult.Failed(failure.error)
    }
}
