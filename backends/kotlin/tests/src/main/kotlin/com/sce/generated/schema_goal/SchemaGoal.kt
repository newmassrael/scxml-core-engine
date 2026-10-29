// SCE-GENERATED — DO NOT EDIT
// source-hash: adbae20e9ca5d62b09870f4a9e5e88119b90dbcf69963958bb021c42c5c2c6a4
// SCE-MAP: schema_goal.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: goal.added
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.schema_goal

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "goal.added"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

data class SchemaGoalPayload(    val id: UInt,    val day: Long?,    val weight: Int)
