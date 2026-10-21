// SCE-GENERATED — DO NOT EDIT
// source-hash: 9eefb6dd8cbce9cedf75b07dbf863e3e812c459c1ce7686309e1f4c36da06a10
// SCE-MAP: forge_call_next.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="transform")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.forge_call_next
import com.sce.generated.forge_call_status.*

fun computeAfter(status: ForgeCallStatus): ForgeCallStatus =
    if (status == ForgeCallStatus.OPEN) ForgeCallStatus.HELD else ForgeCallStatus.DONE
