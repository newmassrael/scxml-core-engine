// SCE-GENERATED — DO NOT EDIT
// source-hash: 43faded87b84ec06f387d0e3d6ccb626e2fa1802d2395c65b70588a4c3c394af
// SCE-MAP: forge_call_weight.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.forge_call_weight
import com.sce.generated.forge_call_status.*
import com.sce.generated.forge_call_level.*

import com.sce.forge.runtime.lookup

private val KEYS: List<ForgeCallStatus> = listOf(ForgeCallStatus.OPEN, ForgeCallStatus.HELD)
private val VALUES: List<ForgeCallLevel> = listOf(ForgeCallLevel.LIGHT, ForgeCallLevel.HEAVY)

fun lookupWeight(status: ForgeCallStatus): ForgeCallLevel? =
    lookup(KEYS, VALUES, status)
