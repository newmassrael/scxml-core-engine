// SCE-GENERATED — DO NOT EDIT
// source-hash: 09d8fec62ddcc806d17c4e1da241b802dbe4f5317b3bd415a705b2ad99d82f3e
// SCE-MAP: event_roster.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.event_roster`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.event_roster

import com.sce.generated.event_member.*

fun eventRoster(roster: List<EventMemberPayload>, node: ULong, role: UByte): com.sce.forge.runtime.AlgorithmResult<List<EventMemberPayload>> {
    // SCE_FORGE.md §3.4.1: a checked operation throws its failure, and this
    // boundary hands it to the caller as a value — never as an exception.
    try {
        val out = ArrayList<EventMemberPayload>(64)
        for (m in roster) {
            if (m.node != node) {
                out.add(m)
            }
        }
        if (role != 0.toUByte()) {
            var made = EventMemberPayload(node = node, role = role)
            out.add(made)
        }
        return com.sce.forge.runtime.AlgorithmResult.Ok(out)
    } catch (failure: com.sce.forge.runtime.AlgorithmFailure) {
        return com.sce.forge.runtime.AlgorithmResult.Failed(failure.error)
    }
}
