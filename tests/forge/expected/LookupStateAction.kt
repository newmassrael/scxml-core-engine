// SCE-MAP: lookup_state_action:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.lookup_state_action

import com.sce.forge.runtime.lookup

private val KEYS: List<Int> = listOf(0, 1, 2, 3)
private val VALUES: List<Int> = listOf(10, 20, 30, 40)

fun lookupAction(state: Int): Int? =
    lookup(KEYS, VALUES, state)
