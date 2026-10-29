// SCE-GENERATED — DO NOT EDIT
// source-hash: 1178ba59796ca1f7c46f8569fd4bfda85c2b97db65f51728f3615d990d97dd1c
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
