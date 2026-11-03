// SCE-GENERATED — DO NOT EDIT
// source-hash: fb4f42a34e7ac40a28d8e5daee5d633c3c45dddac513816edf2bb83fdc85f91c
// SCE-MAP: lww_cell.scxml:37 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: merge.lww_cell
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.lww_cell

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "merge.lww_cell"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

data class LwwCellPayload(    val row: ULong,    val col: ULong,    val value: Long,    val wallTime: Long,    val counter: UInt,    val nodeId: ULong,    val op: UByte,    val aRef: Boolean,    val aRow: ULong,    val aCol: ULong,    val aVal: Long,    val bRef: Boolean,    val bRow: ULong,    val bCol: ULong,    val bVal: Long)
