// SCE-GENERATED — DO NOT EDIT
// source-hash: 2073d8a55462e1ce61d8a77293fdd2c141aceda8ad173303c5b84813ca22eeee
// SCE-MAP: forge_call_next.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="transform")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.forge_call_next
import com.sce.generated.forge_call_status.*

fun computeAfter(status: ForgeCallStatus): ForgeCallStatus =
    if (status == ForgeCallStatus.OPEN) ForgeCallStatus.HELD else ForgeCallStatus.DONE
