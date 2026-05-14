// SCE-MAP: transform_temperature:3

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="transform")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.transform_temperature

fun computeTemperature(raw: UShort): Double =
    raw.toDouble() * 0.1 - 40.0
