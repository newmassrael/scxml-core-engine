// SCE-MAP: interpolation_1d_linear:1 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="interpolation")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.interpolation_1d_linear

import com.sce.forge.runtime.linear

object Interpolation1dLinear {
    private val AXIS_RPM = doubleArrayOf(800.0, 1200.0, 2000.0, 3000.0, 4000.0, 6000.0)
    private val VALUES = doubleArrayOf(120.0, 145.0, 200.0, 230.0, 210.0, 180.0)

    fun lookup(rpm: UShort): Double {
        return linear(
            AXIS_RPM, VALUES,
            rpm.toDouble()
        )
    }
}
