// SCE-GENERATED — DO NOT EDIT
// source-hash: 8de0fa86739ee80dd1cde104a3417a50f438f9eccff41679327b28a2157e9381
// SCE-MAP: forge_call_next.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="transform")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.forge_call_next
import com.sce.generated.forge_call_status.*

fun computeAfter(status: ForgeCallStatus): ForgeCallStatus =
    if (status == ForgeCallStatus.OPEN) ForgeCallStatus.HELD else ForgeCallStatus.DONE
