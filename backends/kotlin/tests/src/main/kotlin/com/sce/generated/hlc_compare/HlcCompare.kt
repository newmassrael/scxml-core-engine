// SCE-GENERATED — DO NOT EDIT
// source-hash: b3df842d016676bf8989832cb164de51027fadf6ca9e7b5543c7bac0c9a13d1a
// SCE-MAP: hlc_compare.scxml:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.hlc_compare`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.hlc_compare

import com.sce.generated.hlc_timestamp.*

fun hlcCompare(a: HlcTimestampPayload, b: HlcTimestampPayload): Int {
    if (a.wallTime != b.wallTime) {
        return if (a.wallTime < b.wallTime) -1 else 1
    }
    if (a.counter != b.counter) {
        return if (a.counter < b.counter) -1 else 1
    }
    if (a.nodeId != b.nodeId) {
        return if (a.nodeId < b.nodeId) -1 else 1
    }
    return 0
}
