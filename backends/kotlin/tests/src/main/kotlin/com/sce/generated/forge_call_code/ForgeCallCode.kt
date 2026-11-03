// SCE-GENERATED — DO NOT EDIT
// source-hash: ef94c112b4026dfab705629195b17ee265cdc572de6e8fceff328262c2af0613
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
