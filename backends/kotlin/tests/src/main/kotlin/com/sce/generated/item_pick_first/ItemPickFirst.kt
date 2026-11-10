// SCE-GENERATED — DO NOT EDIT
// source-hash: 81a3de342b33c3189f4a5813894163bc1e16bfc77aa7ead74e709e8ba7b570b9
// SCE-MAP: algorithm_item_pick_first.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.item_pick_first`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.item_pick_first

import com.sce.generated.schema_item_pick.*

fun itemPickFirst(items: List<SchemaItemPickPayload>, least: Int): SchemaItemPickPayload? {
    var found: SchemaItemPickPayload? = null
    for (it in items) {
        if (found == null && it.count >= least) {
            found = it
        }
    }
    return found
}
