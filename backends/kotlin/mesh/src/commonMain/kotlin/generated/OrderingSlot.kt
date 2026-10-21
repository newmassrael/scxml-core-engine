// SCE-GENERATED — DO NOT EDIT
// source-hash: 3590e0da727b5c60e46e46571a637da3e7beb956941ee2491fcd012174a020af
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

data class OrderingSlotPayload(    val seq: ULong,    val arrivedAtMs: Long)
