// SCE-GENERATED — DO NOT EDIT
// source-hash: ca67d4fa2cf4dbcd6617a3b32561c9b8257bc03aaa65978435303ccf35bae7b3
// SCE-MAP: algorithm_item_status_of.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.item_status_of`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.item_status_of

import com.sce.generated.schema_item.*
import com.sce.generated.enum_item_status.*

fun itemStatusOf(items: List<SchemaItemPayload>, id: UInt): EnumItemStatus {
    var found: EnumItemStatus = EnumItemStatus.OPEN
    for (e in items) {
        if (e.id == id) {
            found = e.status
        }
    }
    return found
}
