// SCE-GENERATED — DO NOT EDIT
// source-hash: adbae20e9ca5d62b09870f4a9e5e88119b90dbcf69963958bb021c42c5c2c6a4
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
