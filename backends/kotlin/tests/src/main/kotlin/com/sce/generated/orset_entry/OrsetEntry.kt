// SCE-GENERATED — DO NOT EDIT
// source-hash: fb4f42a34e7ac40a28d8e5daee5d633c3c45dddac513816edf2bb83fdc85f91c
// SCE-MAP: orset_entry.scxml:19 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: merge.orset_entry
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.orset_entry

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "merge.orset_entry"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

data class OrsetEntryPayload(    val element: ULong,    val wallTime: Long,    val counter: UInt,    val nodeId: ULong)
