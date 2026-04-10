// SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure")
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.procedure_diamond

data class ProcedureResult(val completed: Boolean, val finalState: String)

object ProcedureDiamond {
    private val STATE_NAMES = arrayOf("classify", "high_path", "mid_path", "low_path", "accept", "reject")

    fun execute(sensorValue: UShort, mode: String): ProcedureResult {
        var current = 0
        var iterations = 0
        while (iterations++ < 6) {
            val next = when (current) {
                0 -> {
                    if (sensorValue > 1000.toUShort()) 1
                    else if (sensorValue > 500.toUShort()) 2
                    else 3
                }
                1 -> {
                    if (mode == "strict") 5
                    else 4
                }
                2 -> {
                    4
                }
                3 -> {
                    4
                }
                else -> -1
            }
            if (next < 0) break
            current = next
            if (current == 4 || current == 5) break
        }
        val completed = current == 4 || current == 5
        return ProcedureResult(completed, STATE_NAMES[current])
    }
}
