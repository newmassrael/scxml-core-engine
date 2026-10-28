// SCE-GENERATED — DO NOT EDIT
// source-hash: d11fb6db69c27fed25f47a2f22a5bf7fb78da524d08f0c299e9cc553d32a6375
// SCE-MAP: event_occurrence.scxml:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: calendar.occurrence
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.event_occurrence

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "calendar.occurrence"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

data class EventOccurrencePayload(    val id: ULong,    val start: Long,    val end: Long,    val nth: UInt,    val conflict: UByte)
