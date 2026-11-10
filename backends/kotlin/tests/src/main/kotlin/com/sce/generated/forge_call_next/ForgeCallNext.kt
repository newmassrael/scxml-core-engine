// SCE-GENERATED — DO NOT EDIT
// source-hash: d8c75e4d0233623996dc9a7cc3bdbe850af8d4f91ab023212ebd9f938cf7955f
// SCE-MAP: forge_call_next.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="transform")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.forge_call_next
import com.sce.generated.forge_call_status.*

fun computeAfter(status: ForgeCallStatus): ForgeCallStatus =
    if (status == ForgeCallStatus.OPEN) ForgeCallStatus.HELD else ForgeCallStatus.DONE
