// SCE Forge: Auto-generated from Extended SCXML (sce:kind="condition")
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.condition_range

fun conditionRange(rpm: UInt, minRpm: UInt, maxRpm: UInt): Boolean =
    rpm >= minRpm && rpm <= maxRpm
