// SCE-GENERATED — DO NOT EDIT
// source-hash: 09d8fec62ddcc806d17c4e1da241b802dbe4f5317b3bd415a705b2ad99d82f3e
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
