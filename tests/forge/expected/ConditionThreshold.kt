// SCE Forge: Auto-generated from Extended SCXML (sce:kind="condition")
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.condition_threshold

fun conditionThreshold(coolantTemp: Double, oilTemp: Double, maxTemp: Double): Boolean =
    coolantTemp > maxTemp || oilTemp > maxTemp
