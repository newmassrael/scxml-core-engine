// SCE-GENERATED — DO NOT EDIT
// source-hash: 80708a7c905471af3d2cfb2f6b325d02749fbc7f10a7ba46f908af4c57e34fe1
// source: forge_call_next.scxml
// SCE-MAP: forge_call_next.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="transform")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.forge_call_next
import com.sce.generated.forge_call_status.*

fun computeAfter(status: ForgeCallStatus): ForgeCallStatus =
    if (status == ForgeCallStatus.OPEN) ForgeCallStatus.HELD else ForgeCallStatus.DONE
