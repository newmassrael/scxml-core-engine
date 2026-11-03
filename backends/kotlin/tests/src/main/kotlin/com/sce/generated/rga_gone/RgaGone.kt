// SCE-GENERATED — DO NOT EDIT
// source-hash: fb4f42a34e7ac40a28d8e5daee5d633c3c45dddac513816edf2bb83fdc85f91c
// SCE-MAP: rga_gone.scxml:14 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: merge.rga_gone
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.rga_gone

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "merge.rga_gone"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 58uL

data class RgaGonePayload(    val key: ULong,    val by: ULong)
