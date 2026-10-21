// SCE-GENERATED — DO NOT EDIT
// source-hash: ca67d4fa2cf4dbcd6617a3b32561c9b8257bc03aaa65978435303ccf35bae7b3
// SCE-MAP: event_settled.scxml:17 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.event_settled`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.event_settled

import com.sce.generated.orset_entry.*
import com.sce.generated.merkle_change_hash.*

fun eventSettled(adds: List<OrsetEntryPayload>, tombstones: List<OrsetEntryPayload>, before: Long): com.sce.forge.runtime.AlgorithmResult<UInt> {
    // SCE_FORGE.md §3.4.1: a checked operation throws its failure, and this
    // boundary hands it to the caller as a value — never as an exception.
    try {
        var h: UInt = 0.toUInt()
        for (a in adds) {
            if (a.wallTime <= before) {
                h = h xor com.sce.forge.runtime.SceChecked.take(merkleChangeHash(a, 0.toUByte()))
            }
        }
        for (t in tombstones) {
            if (t.wallTime <= before) {
                h = h xor com.sce.forge.runtime.SceChecked.take(merkleChangeHash(t, 1.toUByte()))
            }
        }
        return com.sce.forge.runtime.AlgorithmResult.Ok(h)
    } catch (failure: com.sce.forge.runtime.AlgorithmFailure) {
        return com.sce.forge.runtime.AlgorithmResult.Failed(failure.error)
    }
}
