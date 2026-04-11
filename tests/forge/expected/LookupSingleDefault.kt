// SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.lookup_single_default

enum class Quality { NONE, LOW, MEDIUM, HIGH }

fun lookupQuality(level: UByte): Quality = when (level.toInt()) {
    3 -> Quality.HIGH
    1 -> Quality.LOW
    2 -> Quality.MEDIUM
    0 -> Quality.NONE
    else -> Quality.NONE
}
