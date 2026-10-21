// SCE-GENERATED — DO NOT EDIT
// source-hash: 0fafd5dc7cb78db436777a56fff2cdefea18c3275c9e740c945e368e87cb13ce
// SCE-MAP: schema_entry.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: ledger.entry
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.schema_entry

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "ledger.entry"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

data class SchemaEntryPayload(    val id: UInt,    val amount: Int)
