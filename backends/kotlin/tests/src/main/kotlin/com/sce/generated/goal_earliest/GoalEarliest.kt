// SCE-GENERATED — DO NOT EDIT
// source-hash: 048fe3b13b4ba344e3f60c0d3a93bf8f8568ab7bd254b4d4763d7e631895e2f6
// SCE-MAP: algorithm_goal_earliest.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.goal_earliest`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.goal_earliest

import com.sce.generated.schema_goal.*

fun goalEarliest(goals: List<SchemaGoalPayload>): Long? {
    var best: Long? = null
    for (g in goals) {
        if (g.day != null && (best == null || com.sce.forge.runtime.SceOptional.proven(g.day) < com.sce.forge.runtime.SceOptional.proven(best))) {
            best = g.day
        }
    }
    return best
}
