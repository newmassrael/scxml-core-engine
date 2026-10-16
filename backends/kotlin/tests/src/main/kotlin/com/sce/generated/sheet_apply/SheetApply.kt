// SCE-GENERATED — DO NOT EDIT
// source-hash: 8149e47f56da0bbb92d7b1c4600956bbfd585c43526372f3618b2e600843296d
// SCE-MAP: sheet_apply.scxml:21 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.sheet_apply`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.sheet_apply

import com.sce.generated.sheet_value.*

fun sheetApply(op: UByte, a: Long, b: Long): com.sce.forge.runtime.AlgorithmResult<SheetValuePayload> {
    // SCE_FORGE.md §3.4.1: a checked operation throws its failure, and this
    // boundary hands it to the caller as a value — never as an exception.
    try {
        var top: Long = 9223372036854775807
        var bottom: Long = com.sce.forge.runtime.SceChecked.sub(com.sce.forge.runtime.SceChecked.neg(top), 1)
        var value: Long = 0
        var fault: UByte = 0.toUByte()
        if (op == 2.toUByte()) {
            if (b > 0 && a > com.sce.forge.runtime.SceChecked.sub(top, b) || b < 0 && a < com.sce.forge.runtime.SceChecked.sub(bottom, b)) {
                fault = 3.toUByte()
            } else {
                value = com.sce.forge.runtime.SceChecked.add(a, b)
            }
        } else {
            if (op == 3.toUByte()) {
                if (b < 0 && a > com.sce.forge.runtime.SceChecked.add(top, b) || b > 0 && a < com.sce.forge.runtime.SceChecked.add(bottom, b)) {
                    fault = 3.toUByte()
                } else {
                    value = com.sce.forge.runtime.SceChecked.sub(a, b)
                }
            } else {
                if (op == 4.toUByte()) {
                    if (a == 0L || b == 0L) {
                        value = 0
                    } else {
                        if (a > 0 && b > 0 && a > com.sce.forge.runtime.SceChecked.div(top, b) || a > 0 && b < 0 && b < com.sce.forge.runtime.SceChecked.div(bottom, a) || a < 0 && b > 0 && a < com.sce.forge.runtime.SceChecked.div(bottom, b) || a < 0 && b < 0 && b < com.sce.forge.runtime.SceChecked.div(top, a)) {
                            fault = 3.toUByte()
                        } else {
                            value = com.sce.forge.runtime.SceChecked.mul(a, b)
                        }
                    }
                } else {
                    if (op == 5.toUByte()) {
                        if (b == 0L) {
                            fault = 2.toUByte()
                        } else {
                            if (a == bottom && b == (-1).toLong()) {
                                fault = 3.toUByte()
                            } else {
                                value = com.sce.forge.runtime.SceChecked.div(a, b)
                            }
                        }
                    } else {
                        fault = 5.toUByte()
                    }
                }
            }
        }
        var result = SheetValuePayload(value = value, fault = fault)
        return com.sce.forge.runtime.AlgorithmResult.Ok(result)
    } catch (failure: com.sce.forge.runtime.AlgorithmFailure) {
        return com.sce.forge.runtime.AlgorithmResult.Failed(failure.error)
    }
}
