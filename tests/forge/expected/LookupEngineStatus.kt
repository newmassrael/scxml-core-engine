// SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.lookup_engine_status

enum class Status { STOP, RUNNING, FAULT }

fun lookupStatus(engSta: UByte): Status = when (engSta.toInt()) {
    0x07 -> Status.FAULT
    0x03 -> Status.RUNNING
    0x00, 0x01, 0x02 -> Status.STOP
    else -> Status.STOP
}