// SCE-GENERATED — DO NOT EDIT
// source-hash: 58e52c40de99a046030dfe178fb69ffa932b0a6a8146307d7430d1e6de80a470
// SCE-MAP: merkle_change_hash.scxml:40 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.merkle_change_hash`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.merkle_change_hash

import com.sce.generated.orset_entry.*

fun merkleChangeHash(change: OrsetEntryPayload, kind: UByte): com.sce.forge.runtime.AlgorithmResult<UInt> {
    // SCE_FORGE.md §3.4.1: a checked operation throws its failure, and this
    // boundary hands it to the caller as a value — never as an exception.
    try {
        if (!(change.wallTime >= 0)) {
            return com.sce.forge.runtime.AlgorithmResult.Failed(com.sce.forge.runtime.AlgorithmError.Precondition)
        }
        var element: ULong = change.element
        var wall: ULong = com.sce.forge.runtime.SceChecked.narrowToULong(change.wallTime)
        var counter: ULong = change.counter.toULong()
        var node: ULong = change.nodeId
        var h: ULong = 0.toULong()
        h = com.sce.forge.runtime.SceChecked.rem(com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(h, 1000003.toULong()), com.sce.forge.runtime.SceChecked.rem(element, 4294967296.toULong())), 4294967296.toULong())
        h = com.sce.forge.runtime.SceChecked.rem(com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(h, 1000003.toULong()), com.sce.forge.runtime.SceChecked.div(element, 4294967296.toULong())), 4294967296.toULong())
        h = com.sce.forge.runtime.SceChecked.rem(com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(h, 1000003.toULong()), com.sce.forge.runtime.SceChecked.rem(wall, 4294967296.toULong())), 4294967296.toULong())
        h = com.sce.forge.runtime.SceChecked.rem(com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(h, 1000003.toULong()), com.sce.forge.runtime.SceChecked.div(wall, 4294967296.toULong())), 4294967296.toULong())
        h = com.sce.forge.runtime.SceChecked.rem(com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(h, 1000003.toULong()), counter), 4294967296.toULong())
        h = com.sce.forge.runtime.SceChecked.rem(com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(h, 1000003.toULong()), com.sce.forge.runtime.SceChecked.rem(node, 4294967296.toULong())), 4294967296.toULong())
        h = com.sce.forge.runtime.SceChecked.rem(com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(h, 1000003.toULong()), com.sce.forge.runtime.SceChecked.div(node, 4294967296.toULong())), 4294967296.toULong())
        h = com.sce.forge.runtime.SceChecked.rem(com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(h, 1000003.toULong()), kind.toULong()), 1.toULong()), 4294967296.toULong())
        h = h xor com.sce.forge.runtime.SceChecked.div(h, 65536.toULong())
        h = com.sce.forge.runtime.SceChecked.rem(com.sce.forge.runtime.SceChecked.mul(h, 2246822507.toULong()), 4294967296.toULong())
        h = h xor com.sce.forge.runtime.SceChecked.div(h, 8192.toULong())
        h = com.sce.forge.runtime.SceChecked.rem(com.sce.forge.runtime.SceChecked.mul(h, 3266489909.toULong()), 4294967296.toULong())
        h = h xor com.sce.forge.runtime.SceChecked.div(h, 65536.toULong())
        return com.sce.forge.runtime.AlgorithmResult.Ok(com.sce.forge.runtime.SceChecked.narrowToUInt(h))
    } catch (failure: com.sce.forge.runtime.AlgorithmFailure) {
        return com.sce.forge.runtime.AlgorithmResult.Failed(failure.error)
    }
}
