// SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure")
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.procedure_linear

data class ProcedureResult(val completed: Boolean, val finalState: String)

object ProcedureLinear {
    private val STATE_NAMES = arrayOf("stage_a", "stage_b", "stage_c", "done")

    fun execute(value: Int): ProcedureResult {
        var current = 0
        var iterations = 0
        while (iterations++ < 4) {
            val next = when (current) {
                0 -> {
                    1
                }
                1 -> {
                    2
                }
                2 -> {
                    3
                }
                else -> -1
            }
            if (next < 0) break
            current = next
            if (current == 3) break
        }
        val completed = current == 3
        return ProcedureResult(completed, STATE_NAMES[current])
    }
}