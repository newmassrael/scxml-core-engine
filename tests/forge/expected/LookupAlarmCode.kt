// SCE-MAP: lookup_alarm_code:6

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.lookup_alarm_code

import com.sce.forge.runtime.lookup

private val KEYS: List<Int> = listOf(100, 200, 300, 400, 500)
private val VALUES: List<Int> = listOf(1, 2, 3, 2, 4)

fun lookupSeverity(code: Int): Int? =
    lookup(KEYS, VALUES, code)
