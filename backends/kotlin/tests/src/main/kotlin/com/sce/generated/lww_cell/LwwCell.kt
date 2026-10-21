// SCE-GENERATED — DO NOT EDIT
// source-hash: b3df842d016676bf8989832cb164de51027fadf6ca9e7b5543c7bac0c9a13d1a
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
