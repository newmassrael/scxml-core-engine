// SCE-GENERATED — DO NOT EDIT
// source-hash: 24f1c455257ea04f598b006a589c3dea642c1ea2e58b2dcf4b49b8cebf3f207e
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
