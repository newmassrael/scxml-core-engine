// SCE-GENERATED — DO NOT EDIT
// source-hash: b2921c03007062914a1e69cc4069c6b54c8d66f97313a50f3246eed3d071c790
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
