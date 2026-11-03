// SCE-GENERATED — DO NOT EDIT
// source-hash: d445777bd638d59bb235766496907e1278e8d6187af708d676fb6e02145c0dea
// SCE-MAP: ordering_slot.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: mesh.ordering_slot
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.ordering_slot

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "mesh.ordering_slot"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 67uL

data class OrderingSlotPayload(    val seq: ULong,    val arrivedAtMs: Long)
