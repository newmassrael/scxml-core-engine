// SCE-GENERATED — DO NOT EDIT
// source-hash: 20f48a3b336c719e7175b6da73bb16a60ea45d33fcaf7f01e6a1ec4ed70af971
// source: algorithm_text_filled.scxml
// SCE-MAP: algorithm_text_filled.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.text_filled`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.text_filled

fun textFilled(text: String): Boolean {
    return text != ""
}
