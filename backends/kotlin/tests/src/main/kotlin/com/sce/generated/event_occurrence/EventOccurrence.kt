// SCE-GENERATED — DO NOT EDIT
// source-hash: 048fe3b13b4ba344e3f60c0d3a93bf8f8568ab7bd254b4d4763d7e631895e2f6
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
