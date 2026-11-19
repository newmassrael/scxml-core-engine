// SCE-GENERATED — DO NOT EDIT
// source-hash: c2e56e3837f180f07a5c044369108d947c57fa7c065dd063d76bf9f37c590bee
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
