// SCE-GENERATED — DO NOT EDIT
// source-hash: fb4f42a34e7ac40a28d8e5daee5d633c3c45dddac513816edf2bb83fdc85f91c
// SCE-MAP: mv_value.scxml:27 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: merge.mv_value
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.mv_value

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "merge.mv_value"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most UTF-8 bytes `text` holds (the bound its type writes, named `merge_text`), as this build resolved it. */
const val TEXT_MAX_SIZE: UInt = 96u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 764uL

data class MvValuePayload(    val id: ULong,    val field: ULong,    val value: Long,    val text: String,    val wallTime: Long,    val counter: UInt,    val nodeId: ULong)
