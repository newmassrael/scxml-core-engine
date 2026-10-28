// SCE-GENERATED — DO NOT EDIT
// source-hash: d11fb6db69c27fed25f47a2f22a5bf7fb78da524d08f0c299e9cc553d32a6375
// SCE-MAP: forge_call_limit.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="interpolation")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.forge_call_limit

import com.sce.forge.runtime.linear

object ForgeCallLimit {
    private val AXIS_RPM = doubleArrayOf(0.0, 1000.0, 2000.0)
    private val VALUES = doubleArrayOf(100.0, 150.0, 120.0)

    fun lookup(rpm: UShort): Double {
        return linear(
            AXIS_RPM, VALUES,
            rpm.toDouble()
        )
    }
}
