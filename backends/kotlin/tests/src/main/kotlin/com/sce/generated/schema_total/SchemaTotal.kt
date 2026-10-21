// SCE-GENERATED — DO NOT EDIT
// source-hash: 09d8fec62ddcc806d17c4e1da241b802dbe4f5317b3bd415a705b2ad99d82f3e
// SCE-MAP: schema_total.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: ledger.total
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.schema_total

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "ledger.total"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

data class SchemaTotalPayload(    val count: UInt,    val sum: Int)
