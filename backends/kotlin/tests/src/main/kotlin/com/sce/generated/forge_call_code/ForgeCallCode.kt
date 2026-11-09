// SCE-GENERATED — DO NOT EDIT
// source-hash: d05ce240c5f3b48b3c83b6bee1ba58feef40d57c310e9a29ef5279604bbaaa42
// SCE-MAP: forge_call_code.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.forge_call_code

import com.sce.forge.runtime.lookup

private val KEYS: List<UByte> = listOf(1u.toUByte(), 2u.toUByte())
private val VALUES: List<Int> = listOf(100, 200)

fun lookupCode(gear: UByte): Int? =
    lookup(KEYS, VALUES, gear)
