// SCE-GENERATED — DO NOT EDIT
// source-hash: b3df842d016676bf8989832cb164de51027fadf6ca9e7b5543c7bac0c9a13d1a
// SCE-MAP: merkle_insert.scxml:33 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.merkle_insert`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.merkle_insert

import com.sce.generated.merkle_node.*

fun merkleInsert(nodes: List<MerkleNodePayload>, minute: Long, hash: UInt): com.sce.forge.runtime.AlgorithmResult<List<MerkleNodePayload>> {
    // SCE_FORGE.md §3.4.1: a checked operation throws its failure, and this
    // boundary hands it to the caller as a value — never as an exception.
    try {
        if (!(minute >= 0 && minute <= 153722867280912)) {
            return com.sce.forge.runtime.AlgorithmResult.Failed(com.sce.forge.runtime.AlgorithmError.Precondition)
        }
        val out = ArrayList<MerkleNodePayload>(2048)
        var pd: UInt = 0.toUInt()
        var pp: ULong = 0.toULong()
        var div: Long = 68630377364883
        var prevDepth: Int = -1
        var prevPrefix: ULong = 0.toULong()
        for (x in nodes) {
            if (!(x.depth <= 30.toUByte() && (x.depth.toInt() > prevDepth || x.depth.toInt() == prevDepth && x.prefix > prevPrefix))) {
                return com.sce.forge.runtime.AlgorithmResult.Failed(com.sce.forge.runtime.AlgorithmError.Precondition)
            }
            prevDepth = x.depth.toInt()
            prevPrefix = x.prefix
            while (pd <= 30.toUInt() && (pd < x.depth.toUInt() || pd == x.depth.toUInt() && pp < x.prefix)) {
                var created = MerkleNodePayload(depth = com.sce.forge.runtime.SceChecked.narrowToUByte((pd).toULong()), prefix = pp, hash = hash)
                out.add(created)
                pd = com.sce.forge.runtime.SceChecked.add(pd, 1.toUInt())
                pp = com.sce.forge.runtime.SceChecked.narrowToULong(com.sce.forge.runtime.SceChecked.div(minute, div))
                if (div > 1) {
                    div = com.sce.forge.runtime.SceChecked.div(div, 3)
                }
            }
            if (pd <= 30.toUInt() && pd == x.depth.toUInt() && pp == x.prefix) {
                var merged = MerkleNodePayload(depth = x.depth, prefix = x.prefix, hash = x.hash xor hash)
                out.add(merged)
                pd = com.sce.forge.runtime.SceChecked.add(pd, 1.toUInt())
                pp = com.sce.forge.runtime.SceChecked.narrowToULong(com.sce.forge.runtime.SceChecked.div(minute, div))
                if (div > 1) {
                    div = com.sce.forge.runtime.SceChecked.div(div, 3)
                }
            } else {
                out.add(x)
            }
        }
        while (pd <= 30.toUInt()) {
            var tail = MerkleNodePayload(depth = com.sce.forge.runtime.SceChecked.narrowToUByte((pd).toULong()), prefix = pp, hash = hash)
            out.add(tail)
            pd = com.sce.forge.runtime.SceChecked.add(pd, 1.toUInt())
            pp = com.sce.forge.runtime.SceChecked.narrowToULong(com.sce.forge.runtime.SceChecked.div(minute, div))
            if (div > 1) {
                div = com.sce.forge.runtime.SceChecked.div(div, 3)
            }
        }
        return com.sce.forge.runtime.AlgorithmResult.Ok(out)
    } catch (failure: com.sce.forge.runtime.AlgorithmFailure) {
        return com.sce.forge.runtime.AlgorithmResult.Failed(failure.error)
    }
}
