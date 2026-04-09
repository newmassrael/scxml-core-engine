// SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure")
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.procedure_startup_check

data class ProcedureResult(val completed: Boolean, val finalState: String)

object ProcedureStartupCheck {
    private val STATE_NAMES = arrayOf("check_voltage", "check_temp", "success", "fail_voltage", "fail_overtemp")

    fun execute(voltage: Float, temperature: Float): ProcedureResult {
        var current = 0
        var iterations = 0
        while (iterations++ < 5) {
            val next = when (current) {
                0 -> {
                    if (voltage >= 11.5 && voltage <= 14.5) 1
                    else 3
                }
                1 -> {
                    if (temperature < 80.0) 2
                    else 4
                }
                else -> -1
            }
            if (next < 0) break
            current = next
            if (current == 2 || current == 3 || current == 4) break
        }
        val completed = current == 2 || current == 3 || current == 4
        return ProcedureResult(completed, STATE_NAMES[current])
    }
}