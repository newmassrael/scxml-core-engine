// SCE-MAP: lookup_unit_scale:6

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.lookup_unit_scale

import com.sce.forge.runtime.lookup

private val KEYS: List<Int> = listOf(1, 2, 3, 4, 5, 6)
private val VALUES: List<Double> = listOf(0.001, 0.01, 0.1, 1.0, 10.0, 100.0)

fun lookupScale(unit: Int): Double? =
    lookup(KEYS, VALUES, unit)
